pub mod api;
pub mod cache;
pub mod exporter;
pub mod models;
pub mod runner;
pub mod scanner;
pub mod tui;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex as AsyncMutex;

use crate::api::{get_metadata_info, load_cached_servers_data, refresh_servers_data, MetadataInfo};
use crate::cache::CacheManager;
use crate::exporter::export_results;
use crate::models::{ServerInfo, TestResult};
use crate::scanner::BulkScanner;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ScanConfig {
    pub port: Option<u16>,
    pub timeout_secs: Option<f64>,
    pub concurrency: Option<usize>,
    pub mode: Option<String>,
    pub country_filter: Option<Vec<String>>,
    pub feature_filter: Option<Vec<String>>,
    pub force_retest: Option<bool>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct ScanProgress {
    pub total: usize,
    pub scanned: usize,
    pub online: usize,
    pub offline: usize,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub speed_eps: f64,
    pub elapsed_secs: f64,
    pub is_running: bool,
    pub is_paused: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct RefreshStatus {
    pub total_servers: usize,
    pub refreshed_at: u64,
    pub age_seconds: u64,
}

pub struct AppState {
    pub cache_manager: CacheManager,
    pub active_scanner: Arc<AsyncMutex<Option<Arc<BulkScanner>>>>,
    pub is_scanning: Arc<AtomicBool>,
    pub cache_dir: PathBuf,
}

fn resolve_cache_dir(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_cache_dir()
        .or_else(|_| app.path().app_data_dir())
        .unwrap_or_else(|_| PathBuf::from(".cache"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// -----------------------------------------------------------------------------
// Tauri Commands Module
// -----------------------------------------------------------------------------
pub mod commands {
    use super::*;

    #[tauri::command]
    pub async fn get_metadata(
        _app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<MetadataInfo, String> {
        Ok(get_metadata_info(Some(&state.cache_dir)))
    }

    #[tauri::command]
    pub async fn refresh_servers(
        _app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<RefreshStatus, String> {
        let (servers, refreshed_at) = refresh_servers_data(Some(&state.cache_dir))
            .await
            .map_err(|e| format!("Failed to refresh servers: {:#}", e))?;

        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(RefreshStatus {
            total_servers: servers.len(),
            refreshed_at,
            age_seconds: now_epoch.saturating_sub(refreshed_at),
        })
    }

    #[tauri::command]
    pub async fn load_servers(
        _app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<Vec<ServerInfo>, String> {
        let cache_info = get_metadata_info(Some(&state.cache_dir));
        if !cache_info.exists || cache_info.count == 0 {
            let (servers, _) = refresh_servers_data(Some(&state.cache_dir))
                .await
                .map_err(|e| format!("Failed to fetch initial servers: {:#}", e))?;
            return Ok(servers);
        }

        load_cached_servers_data(Some(&state.cache_dir))
            .map_err(|e| format!("Failed to read cached servers: {:#}", e))
    }

    #[tauri::command]
    pub async fn get_cached_results(
        state: State<'_, AppState>,
        ttl_secs: Option<f64>,
    ) -> Result<Vec<TestResult>, String> {
        let ttl = ttl_secs.unwrap_or(86400.0);
        let map = state
            .cache_manager
            .get_fresh_cache(ttl)
            .map_err(|e| e.to_string())?;
        Ok(map.into_values().collect())
    }

    #[tauri::command]
    pub async fn clear_results_cache(state: State<'_, AppState>) -> Result<(), String> {
        state.cache_manager.clear_cache().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn start_scan(
        app: AppHandle,
        state: State<'_, AppState>,
        config: ScanConfig,
    ) -> Result<(), String> {
        if state.is_scanning.load(Ordering::SeqCst) {
            if let Some(existing) = state.active_scanner.lock().await.as_ref() {
                existing.stop();
            }
            state.is_scanning.store(false, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if config.force_retest.unwrap_or(false) {
            let _ = state.cache_manager.clear_cache();
        }

        let all_servers = load_cached_servers_data(Some(&state.cache_dir))
            .map_err(|e| format!("Unable to load servers: {:#}", e))?;

        let country_set: Option<std::collections::HashSet<String>> =
            config.country_filter.as_ref().map(|cf| {
                cf.iter()
                    .map(|c| c.trim().to_uppercase())
                    .filter(|c| !c.is_empty() && c != "ALL")
                    .collect()
            });

        let feat_list: Option<Vec<String>> = config.feature_filter.as_ref().map(|ff| {
            ff.iter()
                .map(|f| f.trim().to_lowercase())
                .filter(|f| !f.is_empty() && f != "all")
                .collect()
        });

        let filtered_servers: Vec<ServerInfo> = all_servers
            .into_iter()
            .filter(|s| {
                if let Some(ref cset) = country_set {
                    if !cset.is_empty() && !cset.contains(&s.country_code) {
                        return false;
                    }
                }
                if let Some(ref flist) = feat_list {
                    if !flist.is_empty() && !crate::api::matches_features(s, flist) {
                        return false;
                    }
                }
                true
            })
            .collect();

        if filtered_servers.is_empty() {
            return Err("No servers match the selected criteria to scan.".to_string());
        }

        let port = config.port.unwrap_or(443);
        let timeout_secs = config.timeout_secs.unwrap_or(30.0);
        let concurrency = config.concurrency.unwrap_or(200);
        let mode = config.mode.unwrap_or_else(|| "tcp".to_string());

        let scanner = Arc::new(BulkScanner::new(
            filtered_servers.clone(),
            state.cache_manager.clone(),
            concurrency,
            port,
            timeout_secs,
            &mode,
        ));

        {
            let mut guard = state.active_scanner.lock().await;
            *guard = Some(scanner.clone());
        }

        state.is_scanning.store(true, Ordering::SeqCst);

        let is_scanning_flag = state.is_scanning.clone();
        let scanner_runner = scanner.clone();
        let app_handle = app.clone();
        let total = filtered_servers.len();

        tokio::spawn(async move {
            let scanned_count = Arc::new(AtomicUsize::new(0));
            let online_count = Arc::new(AtomicUsize::new(0));
            let offline_count = Arc::new(AtomicUsize::new(0));
            let total_latency = Arc::new(AsyncMutex::new(0.0));
            let min_latency = Arc::new(AsyncMutex::new(f64::MAX));
            let max_latency = Arc::new(AsyncMutex::new(0.0));

            let start_time = Instant::now();
            let app_handle_for_events = app_handle.clone();
            let scanned_for_cb = scanned_count.clone();
            let online_for_cb = online_count.clone();
            let offline_for_cb = offline_count.clone();
            let total_lat_cb = total_latency.clone();
            let min_lat_cb = min_latency.clone();
            let max_lat_cb = max_latency.clone();

            let app_handle_for_tick = app_handle.clone();
            let scanned_for_tick = scanned_count.clone();
            let online_for_tick = online_count.clone();
            let offline_for_tick = offline_count.clone();
            let min_lat_tick = min_latency.clone();
            let max_lat_tick = max_latency.clone();
            let total_lat_tick = total_latency.clone();
            let is_scanning_for_tick = is_scanning_flag.clone();
            let scanner_for_tick = scanner_runner.clone();

            let progress_task = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(150));
                while is_scanning_for_tick.load(Ordering::Relaxed) {
                    interval.tick().await;

                    let s = scanned_for_tick.load(Ordering::Relaxed);
                    let on = online_for_tick.load(Ordering::Relaxed);
                    let off = offline_for_tick.load(Ordering::Relaxed);
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 { s as f64 / elapsed } else { 0.0 };

                    let min_lat = *min_lat_tick.lock().await;
                    let max_lat = *max_lat_tick.lock().await;
                    let sum_lat = *total_lat_tick.lock().await;
                    let avg_lat = if on > 0 { sum_lat / (on as f64) } else { 0.0 };

                    let p = ScanProgress {
                        total,
                        scanned: s,
                        online: on,
                        offline: off,
                        avg_latency_ms: (avg_lat * 100.0).round() / 100.0,
                        min_latency_ms: if min_lat == f64::MAX { 0.0 } else { min_lat },
                        max_latency_ms: max_lat,
                        speed_eps: (speed * 10.0).round() / 10.0,
                        elapsed_secs: (elapsed * 10.0).round() / 10.0,
                        is_running: true,
                        is_paused: scanner_for_tick.is_paused(),
                    };

                    let _ = app_handle_for_tick.emit("scan-progress", p);
                }
            });

            scanner_runner
                .run(move |result| {
                    scanned_for_cb.fetch_add(1, Ordering::Relaxed);
                    if result.status == crate::models::TestStatus::Online {
                        online_for_cb.fetch_add(1, Ordering::Relaxed);
                        if let Some(lat) = result.latency_ms {
                            let total_ref = total_lat_cb.clone();
                            let min_ref = min_lat_cb.clone();
                            let max_ref = max_lat_cb.clone();
                            tokio::spawn(async move {
                                let mut sum = total_ref.lock().await;
                                *sum += lat;
                                let mut min = min_ref.lock().await;
                                if lat < *min {
                                    *min = lat;
                                }
                                let mut max = max_ref.lock().await;
                                if lat > *max {
                                    *max = lat;
                                }
                            });
                        }
                    } else {
                        offline_for_cb.fetch_add(1, Ordering::Relaxed);
                    }

                    let _ = app_handle_for_events.emit("scan-result", result);
                })
                .await;

            is_scanning_flag.store(false, Ordering::SeqCst);
            let _ = progress_task.await;

            let s = scanned_count.load(Ordering::Relaxed);
            let on = online_count.load(Ordering::Relaxed);
            let off = offline_count.load(Ordering::Relaxed);
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 { s as f64 / elapsed } else { 0.0 };
            let min_lat = *min_latency.lock().await;
            let max_lat = *max_latency.lock().await;
            let sum_lat = *total_latency.lock().await;
            let avg_lat = if on > 0 { sum_lat / (on as f64) } else { 0.0 };

            let final_progress = ScanProgress {
                total,
                scanned: s,
                online: on,
                offline: off,
                avg_latency_ms: (avg_lat * 100.0).round() / 100.0,
                min_latency_ms: if min_lat == f64::MAX { 0.0 } else { min_lat },
                max_latency_ms: max_lat,
                speed_eps: (speed * 10.0).round() / 10.0,
                elapsed_secs: (elapsed * 10.0).round() / 10.0,
                is_running: false,
                is_paused: false,
            };

            let _ = app_handle.emit("scan-progress", &final_progress);
            let _ = app_handle.emit("scan-finished", ());
        });

        Ok(())
    }

    #[tauri::command]
    pub async fn stop_scan(state: State<'_, AppState>) -> Result<(), String> {
        if let Some(sc) = state.active_scanner.lock().await.as_ref() {
            sc.stop();
        }
        state.is_scanning.store(false, Ordering::SeqCst);
        Ok(())
    }

    #[tauri::command]
    pub async fn toggle_pause_scan(state: State<'_, AppState>) -> Result<bool, String> {
        if let Some(sc) = state.active_scanner.lock().await.as_ref() {
            Ok(sc.toggle_pause())
        } else {
            Ok(false)
        }
    }

    #[tauri::command]
    pub async fn export_results_file(
        results: Vec<TestResult>,
        format: String,
        target_path: Option<String>,
    ) -> Result<String, String> {
        let path_str = target_path.unwrap_or_else(|| {
            if format.eq_ignore_ascii_case("json") {
                "nord_results.json".to_string()
            } else {
                "nord_results.csv".to_string()
            }
        });

        let is_json = format.eq_ignore_ascii_case("json");
        let path = Path::new(&path_str);
        let (csv_path, json_path) = if is_json {
            (None, Some(path))
        } else {
            (Some(path), None)
        };

        export_results(&results, csv_path, json_path)
            .map_err(|e| format!("Export failed: {:#}", e))?;

        Ok(path_str)
    }

    #[tauri::command]
    pub fn get_app_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

// -----------------------------------------------------------------------------
// App Entrypoint
// -----------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Auto updater plugin
    builder = builder.plugin(tauri_plugin_updater::Builder::new().build());

    // Single-instance plugin for desktop platforms (not mobile)
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .setup(|app| {
            let cache_dir = resolve_cache_dir(&app.handle());
            let db_path = cache_dir.join("nord_test_cache.db");
            let cache_mgr = CacheManager::new(Some(&db_path))
                .map_err(|e| e.to_string())?;

            app.manage(AppState {
                cache_manager: cache_mgr,
                active_scanner: Arc::new(AsyncMutex::new(None)),
                is_scanning: Arc::new(AtomicBool::new(false)),
                cache_dir,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_metadata,
            commands::refresh_servers,
            commands::load_servers,
            commands::get_cached_results,
            commands::clear_results_cache,
            commands::start_scan,
            commands::stop_scan,
            commands::toggle_pause_scan,
            commands::export_results_file,
            commands::get_app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
