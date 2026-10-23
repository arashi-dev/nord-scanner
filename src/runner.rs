use anyhow::Result;
use crossterm::{
    event::{Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use crate::api::fetch_all_servers;
use crate::cache::CacheManager;
use crate::exporter::export_results;
use crate::models::{ServerInfo, TestResult};
use crate::scanner::BulkScanner;
use crate::tui::{render_ui, AppState};

pub fn parse_ttl(ttl_input: &str) -> f64 {
    let s = ttl_input.trim().to_lowercase();
    if let Some(val) = s.strip_suffix('s') {
        val.parse::<f64>().unwrap_or(3600.0).max(1.0)
    } else if let Some(val) = s.strip_suffix('m') {
        (val.parse::<f64>().unwrap_or(60.0) * 60.0).max(1.0)
    } else if let Some(val) = s.strip_suffix('h') {
        (val.parse::<f64>().unwrap_or(6.0) * 3600.0).max(1.0)
    } else if let Some(val) = s.strip_suffix('d') {
        (val.parse::<f64>().unwrap_or(1.0) * 86400.0).max(1.0)
    } else {
        s.parse::<f64>().unwrap_or(21600.0).max(1.0)
    }
}

pub struct AppRunner {
    pub concurrency: usize,
    pub port: u16,
    pub timeout: f64,
    pub ttl_str: String,
    pub ttl_seconds: f64,
    pub force_retest: bool,
    pub refresh_servers: bool,
    pub mode: String,
    pub country_filter: Option<Vec<String>>,
    pub feature_filter: Option<Vec<String>>,
    pub csv_export: Option<String>,
    pub json_export: Option<String>,
    pub auto_exit: bool,
}

impl AppRunner {
    pub async fn run(self) -> Result<()> {
        let filter_desc = self
            .country_filter
            .as_ref()
            .map(|c| c.join(",").to_uppercase())
            .unwrap_or_else(|| "ALL".to_string());

        let type_desc = self
            .feature_filter
            .as_ref()
            .map(|f| f.join("+"))
            .unwrap_or_else(|| "ALL".to_string());

        println!("\x1b[1;36mInitializing NordVPN Bulk Tester...\x1b[0m");
        println!(
            "\x1b[2mCountry filter: {} | Type filter: {} | Mode: {} | Concurrency: {}\x1b[0m",
            filter_desc,
            type_desc,
            self.mode.to_uppercase(),
            self.concurrency
        );

        let cache_manager = CacheManager::new(None)?;

        if self.force_retest {
            cache_manager.clear_cache()?;
            println!("\x1b[33mCache cleared (--force specified). Retesting all servers.\x1b[0m");
        }

        let all_servers = fetch_all_servers(
            self.refresh_servers,
            |msg| println!("\x1b[2m{}\x1b[0m", msg),
            self.country_filter.as_deref(),
            self.feature_filter.as_deref(),
        )
        .await?;

        if all_servers.is_empty() {
            eprintln!("\x1b[1;31mNo servers matched the criteria or failed to load server list.\x1b[0m");
            return Ok(());
        }

        // Check fresh cache
        let fresh_cache = cache_manager.get_fresh_cache(self.ttl_seconds)?;
        let relevant_cache: std::collections::HashMap<i64, TestResult> = all_servers
            .iter()
            .filter_map(|s| fresh_cache.get(&s.id).map(|r| (s.id, r.clone())))
            .collect();

        let cached_count = relevant_cache.len();
        let pending_count = all_servers.len().saturating_sub(cached_count);

        let app_state = Arc::new(Mutex::new(AppState::new(
            all_servers.len(),
            cached_count,
            pending_count,
            self.concurrency,
            self.port,
            self.ttl_str.clone(),
            self.mode.clone(),
            filter_desc.clone(),
            type_desc.clone(),
        )));

        app_state
            .lock()
            .unwrap()
            .register_cached_results(relevant_cache);

        // Terminal setup
        enable_raw_mode()?;
        let mut stdout_handle = stdout();
        execute!(stdout_handle, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout_handle);
        let terminal = Arc::new(Mutex::new(Terminal::new(backend)?));

        // Panic hook to restore terminal
        let default_panic = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = execute!(stdout(), LeaveAlternateScreen);
            default_panic(panic_info);
        }));

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let retest_flag = Arc::new(AtomicBool::new(false));

        let mut event_reader = crossterm::event::EventStream::new();
        let mut render_interval = tokio::time::interval(Duration::from_millis(100));

        let mut untested_servers: Vec<ServerInfo> = all_servers
            .iter()
            .filter(|s| !fresh_cache.contains_key(&s.id))
            .cloned()
            .collect();

        let csv_export_path = self.csv_export.clone();
        let json_export_path = self.json_export.clone();

        'session_loop: while !shutdown_flag.load(Ordering::SeqCst) {
            retest_flag.store(false, Ordering::SeqCst);

            let scanner_opt = if !untested_servers.is_empty() {
                let actual_concurrency = self.concurrency.min(untested_servers.len()).max(1);
                let scanner = Arc::new(BulkScanner::new(
                    untested_servers.clone(),
                    cache_manager.clone(),
                    actual_concurrency,
                    self.port,
                    self.timeout,
                    &self.mode,
                ));

                {
                    let mut state = app_state.lock().unwrap();
                    state.status_message = format!(
                        "Scanning {} servers ({} workers)...",
                        untested_servers.len(),
                        actual_concurrency
                    );
                }

                let state_clone = app_state.clone();
                let sc_clone = scanner.clone();
                let scan_task = tokio::spawn(async move {
                    sc_clone
                        .run(move |result| {
                            state_clone.lock().unwrap().record_result(result);
                        })
                        .await;
                });

                Some((scanner, scan_task))
            } else {
                let mut state = app_state.lock().unwrap();
                state.is_completed = true;
                state.status_message =
                    "All servers fresh in cache! Press [R] to force retest.".to_string();
                None
            };

            // Main interaction loop
            let mut scan_finished = scanner_opt.is_none();
            while !shutdown_flag.load(Ordering::SeqCst) && !retest_flag.load(Ordering::SeqCst) {
                tokio::select! {
                    _ = render_interval.tick() => {
                        let state = app_state.lock().unwrap();
                        let mut term = terminal.lock().unwrap();
                        let _ = term.draw(|f| render_ui(f, &state));
                    }
                    maybe_event = event_reader.next() => {
                        if let Some(Ok(event)) = maybe_event {
                            if let Event::Key(key_event) = event {
                                match key_event.code {
                                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                                        app_state.lock().unwrap().status_message = "Exiting and saving results...".to_string();
                                        shutdown_flag.store(true, Ordering::SeqCst);
                                        if let Some((ref sc, _)) = scanner_opt {
                                            sc.stop();
                                        }
                                        break;
                                    }
                                    KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                                        app_state.lock().unwrap().status_message = "Interrupted by user. Exiting...".to_string();
                                        shutdown_flag.store(true, Ordering::SeqCst);
                                        if let Some((ref sc, _)) = scanner_opt {
                                            sc.stop();
                                        }
                                        break;
                                    }
                                    KeyCode::Char('p') | KeyCode::Char('P') => {
                                        if let Some((ref sc, _)) = scanner_opt {
                                            let is_paused = sc.toggle_pause();
                                            let mut state = app_state.lock().unwrap();
                                            state.is_paused = is_paused;
                                            state.status_message = if is_paused {
                                                "PAUSED (press [P] to resume)".to_string()
                                            } else {
                                                "Resumed testing...".to_string()
                                            };
                                        }
                                    }
                                    KeyCode::Char('r') | KeyCode::Char('R') => {
                                        app_state.lock().unwrap().status_message = "Forcing full re-test: clearing cache...".to_string();
                                        retest_flag.store(true, Ordering::SeqCst);
                                        if let Some((ref sc, _)) = scanner_opt {
                                            sc.stop();
                                        }
                                        break;
                                    }
                                    KeyCode::Char('e') | KeyCode::Char('E') => {
                                        let results: Vec<TestResult> = app_state.lock().unwrap().results_map.values().cloned().collect();
                                        if results.is_empty() {
                                            app_state.lock().unwrap().status_message = "No results to export yet.".to_string();
                                        } else {
                                            let csv_p = csv_export_path.as_deref().map(Path::new);
                                            let json_p = json_export_path.as_deref().map(Path::new);
                                            match export_results(&results, csv_p, json_p) {
                                                Ok(paths) => {
                                                    let names: Vec<String> = paths.values().filter_map(|p| Path::new(p).file_name().and_then(|n| n.to_str()).map(String::from)).collect();
                                                    app_state.lock().unwrap().status_message = format!("Saved: {}", names.join(", "));
                                                }
                                                Err(e) => {
                                                    app_state.lock().unwrap().status_message = format!("Export failed: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ = async {
                        if let Some((_, ref handle)) = scanner_opt {
                            if !scan_finished && handle.is_finished() {
                                scan_finished = true;
                                let mut state = app_state.lock().unwrap();
                                state.is_completed = true;
                                state.status_message = "Scan complete! Results cached.".to_string();
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    } => {
                        if scan_finished && self.auto_exit {
                            break 'session_loop;
                        }
                    }
                }
            }

            if retest_flag.load(Ordering::SeqCst) {
                let _ = cache_manager.clear_cache();
                *app_state.lock().unwrap() = AppState::new(
                    all_servers.len(),
                    0,
                    all_servers.len(),
                    self.concurrency,
                    self.port,
                    self.ttl_str.clone(),
                    self.mode.clone(),
                    filter_desc.clone(),
                    type_desc.clone(),
                );
                untested_servers = all_servers.clone();
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        let mut term = terminal.lock().unwrap();
        execute!(term.backend_mut(), LeaveAlternateScreen)?;
        term.show_cursor()?;
        drop(term);

        // Final export
        let results: Vec<TestResult> = app_state
            .lock()
            .unwrap()
            .results_map
            .values()
            .cloned()
            .collect();
        if !results.is_empty() {
            let csv_p = csv_export_path.as_deref().map(Path::new);
            let json_p = json_export_path.as_deref().map(Path::new);
            let _ = export_results(&results, csv_p, json_p);
        }

        // Print final summary
        let state = app_state.lock().unwrap();
        println!("\n\x1b[1;36m=== NordVPN Test Run Complete ===\x1b[0m");
        println!("Total Servers:   {}", state.total_servers);
        println!("Passed (Cached): {}", state.cached_count);
        println!("Online:          \x1b[32m{}\x1b[0m", state.online_count);
        println!("Offline/Timeout: \x1b[31m{}\x1b[0m", state.offline_count);
        if let Some(min) = state.min_latency() {
            let avg = state.avg_latency().unwrap_or(0.0);
            let max = state.max_latency().unwrap_or(0.0);
            println!(
                "Latency:         Min: \x1b[32m{:.1}ms\x1b[0m | Avg: \x1b[36m{:.1}ms\x1b[0m | Max: \x1b[35m{:.1}ms\x1b[0m",
                min, avg, max
            );
        }
        if let Some(ref p) = csv_export_path {
            if Path::new(p).exists() {
                println!(
                    "CSV Export:      \x1b[2m{}\x1b[0m",
                    Path::new(p).canonicalize().unwrap_or_else(|_| PathBuf::from(p)).display()
                );
            }
        }
        if let Some(ref p) = json_export_path {
            if Path::new(p).exists() {
                println!(
                    "JSON Export:     \x1b[2m{}\x1b[0m",
                    Path::new(p).canonicalize().unwrap_or_else(|_| PathBuf::from(p)).display()
                );
            }
        }

        Ok(())
    }
}
