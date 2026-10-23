use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::models::{TestResult, TestStatus};

#[derive(Serialize)]
struct JsonExportRoot<'a> {
    metadata: JsonMetadata,
    servers: Vec<JsonServer<'a>>,
}

#[derive(Serialize)]
struct JsonMetadata {
    exported_at: String,
    total_servers: usize,
    online_servers: usize,
    offline_servers: usize,
    avg_latency_ms: Option<f64>,
}

#[derive(Serialize)]
struct JsonServer<'a> {
    rank: Option<usize>,
    id: i64,
    name: &'a str,
    hostname: &'a str,
    ip: &'a str,
    port: u16,
    country: &'a str,
    country_code: &'a str,
    city: &'a str,
    features: &'a [String],
    status: &'static str,
    latency_ms: Option<f64>,
    load: i32,
    is_cached: bool,
    tested_at: f64,
    error: Option<&'a str>,
}

pub fn export_results(
    results: &[TestResult],
    csv_path: Option<&Path>,
    json_path: Option<&Path>,
) -> Result<HashMap<String, String>> {
    let mut sorted_results = results.to_vec();
    sorted_results.sort_by(|a, b| {
        let a_online = a.status == TestStatus::Online && a.latency_ms.is_some();
        let b_online = b.status == TestStatus::Online && b.latency_ms.is_some();

        match (a_online, b_online) {
            (true, true) => {
                let la = a.latency_ms.unwrap();
                let lb = b.latency_ms.unwrap();
                la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
            }
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (false, false) => a.server_id.cmp(&b.server_id),
        }
    });

    let mut out_paths = HashMap::new();

    if let Some(csv_p) = csv_path {
        if let Some(parent) = csv_p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let file = File::create(csv_p)
            .with_context(|| format!("Failed to create CSV file at {:?}", csv_p))?;
        let mut wtr = csv::Writer::from_writer(file);

        wtr.write_record([
            "Rank",
            "Server ID",
            "Name",
            "Hostname",
            "IP Address",
            "Port",
            "Country Code",
            "Country",
            "City",
            "Features",
            "Status",
            "Latency (ms)",
            "Load (%)",
            "Is Cached",
            "Tested At",
            "Error",
        ])?;

        for (i, r) in sorted_results.iter().enumerate() {
            let rank_str = if r.status == TestStatus::Online {
                (i + 1).to_string()
            } else {
                "-".to_string()
            };

            let dt = Local
                .timestamp_opt(r.tested_at as i64, 0)
                .single()
                .unwrap_or_else(|| Local::now());
            let tested_at_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();

            let features_str = if r.features.is_empty() {
                "Standard".to_string()
            } else {
                r.features.join(", ")
            };

            let lat_str = match r.latency_ms {
                Some(lat) => format!("{:.2}", lat),
                None => String::new(),
            };

            let server_id_str = r.server_id.to_string();
            let port_str = r.port.to_string();
            let load_str = r.load.to_string();
            let cached_str = if r.is_cached { "Yes" } else { "No" };

            wtr.write_record([
                rank_str.as_str(),
                server_id_str.as_str(),
                r.name.as_str(),
                r.hostname.as_str(),
                r.station.as_str(),
                port_str.as_str(),
                r.country_code.as_str(),
                r.country_name.as_str(),
                r.city.as_str(),
                features_str.as_str(),
                r.status.as_str(),
                lat_str.as_str(),
                load_str.as_str(),
                cached_str,
                tested_at_str.as_str(),
                r.error_msg.as_deref().unwrap_or(""),
            ])?;
        }

        wtr.flush()?;
        out_paths.insert("csv".to_string(), csv_p.display().to_string());
    }

    if let Some(json_p) = json_path {
        if let Some(parent) = json_p.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let online_count = sorted_results
            .iter()
            .filter(|r| r.status == TestStatus::Online)
            .count();
        let latencies: Vec<f64> = sorted_results
            .iter()
            .filter_map(|r| {
                if r.status == TestStatus::Online {
                    r.latency_ms
                } else {
                    None
                }
            })
            .collect();

        let avg_latency_ms = if !latencies.is_empty() {
            let sum: f64 = latencies.iter().sum();
            Some((sum / latencies.len() as f64 * 100.0).round() / 100.0)
        } else {
            None
        };

        let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let servers: Vec<JsonServer> = sorted_results
            .iter()
            .enumerate()
            .map(|(i, r)| JsonServer {
                rank: if r.status == TestStatus::Online {
                    Some(i + 1)
                } else {
                    None
                },
                id: r.server_id,
                name: &r.name,
                hostname: &r.hostname,
                ip: &r.station,
                port: r.port,
                country: &r.country_name,
                country_code: &r.country_code,
                city: &r.city,
                features: &r.features,
                status: r.status.as_str(),
                latency_ms: r.latency_ms,
                load: r.load,
                is_cached: r.is_cached,
                tested_at: r.tested_at,
                error: r.error_msg.as_deref(),
            })
            .collect();

        let root = JsonExportRoot {
            metadata: JsonMetadata {
                exported_at: now_str,
                total_servers: sorted_results.len(),
                online_servers: online_count,
                offline_servers: sorted_results.len() - online_count,
                avg_latency_ms,
            },
            servers,
        };

        let json_data = serde_json::to_string_pretty(&root)?;
        let mut file = File::create(json_p)
            .with_context(|| format!("Failed to create JSON file at {:?}", json_p))?;
        file.write_all(json_data.as_bytes())?;
        out_paths.insert("json".to_string(), json_p.display().to_string());
    }

    Ok(out_paths)
}
