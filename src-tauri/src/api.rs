use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::ServerInfo;

pub const NORD_SERVERS_URL: &str = "https://api.nordvpn.com/v1/servers?limit=0";
pub const META_CACHE_TTL_SECS: u64 = 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataInfo {
    pub exists: bool,
    pub count: usize,
    pub last_modified: Option<u64>,
    pub age_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RawServer {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    hostname: Option<String>,
    #[serde(default)]
    station: Option<String>,
    #[serde(default)]
    load: Option<i32>,
    #[serde(default)]
    locations: Option<Vec<RawLocation>>,
    #[serde(default)]
    technologies: Option<Vec<RawIdentifier>>,
    #[serde(default)]
    groups: Option<Vec<RawIdentifier>>,
}

#[derive(Debug, Deserialize)]
struct RawLocation {
    #[serde(default)]
    country: Option<RawCountry>,
    #[serde(default)]
    city: Option<RawCity>,
}

#[derive(Debug, Deserialize)]
struct RawCountry {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawCity {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawIdentifier {
    #[serde(default)]
    identifier: Option<String>,
}

fn parse_server(s: RawServer) -> Option<ServerInfo> {
    let station = s.station?;
    if station.trim().is_empty() {
        return None;
    }

    let (country_name, country_code, city_name) = match s.locations.and_then(|mut locs| {
        if locs.is_empty() {
            None
        } else {
            Some(locs.remove(0))
        }
    }) {
        Some(loc) => {
            let country = loc.country.unwrap_or(RawCountry {
                name: None,
                code: None,
            });
            let city = loc.city.unwrap_or(RawCity { name: None });
            (
                country.name.unwrap_or_else(|| "Unknown".to_string()),
                country.code.unwrap_or_else(|| "??".to_string()).to_uppercase(),
                city.name.unwrap_or_default(),
            )
        }
        None => ("Unknown".to_string(), "??".to_string(), String::new()),
    };

    let technologies = s
        .technologies
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| t.identifier)
        .collect();

    let groups = s
        .groups
        .unwrap_or_default()
        .into_iter()
        .filter_map(|g| g.identifier)
        .collect();

    let id = s.id;
    let name = s.name.unwrap_or_else(|| format!("Server #{}", id));
    let hostname = s.hostname.unwrap_or_default();
    let load = s.load.unwrap_or(0);

    Some(ServerInfo {
        id,
        name,
        hostname,
        station,
        country_name,
        country_code,
        city: city_name,
        load,
        technologies,
        groups,
    })
}

pub fn matches_features(server: &ServerInfo, requested_features: &[String]) -> bool {
    for rf in requested_features {
        let f = rf.trim().to_lowercase();
        if (f == "obfuscated" || f == "xor" || f == "obfuscate") && server.is_obfuscated() {
            return true;
        }
        if (f == "nordwhisper" || f == "whisper" || f == "nord_whisper")
            && server.is_nordwhisper()
        {
            return true;
        }
        if f == "p2p" && server.groups.iter().any(|g| g.to_lowercase().contains("p2p")) {
            return true;
        }
        if (f == "double" || f == "double_vpn")
            && server.groups.iter().any(|g| g.to_lowercase().contains("double"))
        {
            return true;
        }
        if f == "onion"
            && server.groups.iter().any(|g| g.to_lowercase().contains("onion"))
        {
            return true;
        }
        if f == "dedicated"
            && server
                .groups
                .iter()
                .any(|g| g.to_lowercase().contains("dedicated"))
        {
            return true;
        }
        // Generic match against technologies or groups
        if server.technologies.iter().any(|t| t.to_lowercase().contains(&f))
            || server.groups.iter().any(|g| g.to_lowercase().contains(&f))
        {
            return true;
        }
    }
    false
}

pub fn get_cache_file(custom_dir: Option<&Path>) -> PathBuf {
    let dir = match custom_dir {
        Some(d) => d.to_path_buf(),
        None => PathBuf::from(".cache"),
    };
    let _ = fs::create_dir_all(&dir);
    dir.join("nord_servers_metadata.json")
}

pub fn get_metadata_info(custom_dir: Option<&Path>) -> MetadataInfo {
    let cache_file = get_cache_file(custom_dir);
    if !cache_file.exists() {
        return MetadataInfo {
            exists: false,
            count: 0,
            last_modified: None,
            age_seconds: None,
        };
    }

    let meta = fs::metadata(&cache_file).ok();
    let mtime_epoch = meta.and_then(|m| m.modified().ok()).and_then(|t| {
        t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
    });

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let age_seconds = mtime_epoch.map(|mt| now_epoch.saturating_sub(mt));

    // Try reading count
    let count = if let Ok(data) = fs::read(&cache_file) {
        if let Ok(raw_list) = serde_json::from_slice::<Vec<RawServer>>(&data) {
            raw_list.len()
        } else {
            0
        }
    } else {
        0
    };

    MetadataInfo {
        exists: count > 0,
        count,
        last_modified: mtime_epoch,
        age_seconds,
    }
}

pub async fn refresh_servers_data(custom_dir: Option<&Path>) -> Result<(Vec<ServerInfo>, u64)> {
    let cache_file = get_cache_file(custom_dir);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()?;

    let resp = client.get(NORD_SERVERS_URL).send().await
        .context("Failed to connect to NordVPN API")?;
    let bytes = resp.bytes().await.context("Failed to read server response bytes")?;
    let raw_bytes = bytes.to_vec();

    let raw_list: Vec<RawServer> = serde_json::from_slice(&raw_bytes)
        .context("Failed to deserialize NordVPN server catalog JSON")?;

    let _ = fs::write(&cache_file, &raw_bytes);

    let servers: Vec<ServerInfo> = raw_list.into_iter().filter_map(parse_server).collect();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

    Ok((servers, now))
}

pub fn load_cached_servers_data(custom_dir: Option<&Path>) -> Result<Vec<ServerInfo>> {
    let cache_file = get_cache_file(custom_dir);
    let bytes = fs::read(&cache_file).context("No cached servers file found")?;
    let raw_list: Vec<RawServer> = serde_json::from_slice(&bytes)
        .context("Failed to deserialize NordVPN server catalog JSON")?;

    let servers: Vec<ServerInfo> = raw_list.into_iter().filter_map(parse_server).collect();
    Ok(servers)
}

pub async fn fetch_all_servers<F>(
    force_refresh: bool,
    progress_callback: F,
    country_filter: Option<&[String]>,
    feature_filter: Option<&[String]>,
) -> Result<Vec<ServerInfo>>
where
    F: Fn(&str),
{
    let cache_file = get_cache_file(None);
    let mut raw_bytes: Option<Vec<u8>> = None;

    if !force_refresh && cache_file.exists() {
        if let Ok(metadata) = fs::metadata(&cache_file) {
            if let Ok(mtime) = metadata.modified() {
                if let Ok(age) = SystemTime::now().duration_since(mtime) {
                    if age.as_secs() < META_CACHE_TTL_SECS {
                        let age_hours = age.as_secs_f64() / 3600.0;
                        progress_callback(&format!(
                            "Loading server metadata from local cache ({:.1}h old)...",
                            age_hours
                        ));
                        if let Ok(data) = fs::read(&cache_file) {
                            raw_bytes = Some(data);
                        }
                    }
                }
            }
        }
    }

    if raw_bytes.is_none() {
        progress_callback("Connecting to NordVPN API to fetch complete server catalog...");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        match client.get(NORD_SERVERS_URL).send().await {
            Ok(resp) => {
                if let Ok(bytes) = resp.bytes().await {
                    let vec_bytes = bytes.to_vec();
                    let _ = fs::write(&cache_file, &vec_bytes);
                    raw_bytes = Some(vec_bytes);
                }
            }
            Err(e) => {
                if cache_file.exists() {
                    progress_callback(&format!(
                        "API fetch failed ({}). Falling back to cached servers...",
                        e
                    ));
                    raw_bytes = fs::read(&cache_file).ok();
                } else {
                    return Err(anyhow::anyhow!(
                        "Failed to fetch NordVPN servers and no cache available: {}",
                        e
                    ));
                }
            }
        }
    }

    let bytes = raw_bytes.context("No server metadata found")?;
    let raw_list: Vec<RawServer> = serde_json::from_slice(&bytes)
        .context("Failed to deserialize NordVPN server catalog JSON")?;

    let country_set: Option<HashSet<String>> = country_filter.map(|cf| {
        cf.iter()
            .map(|c| c.trim().to_uppercase())
            .filter(|c| !c.is_empty())
            .collect()
    });

    let feat_list: Option<Vec<String>> = feature_filter.map(|ff| {
        ff.iter()
            .map(|f| f.trim().to_lowercase())
            .filter(|f| !f.is_empty())
            .collect()
    });

    let mut servers = Vec::with_capacity(raw_list.len());

    for item in raw_list {
        if let Some(parsed) = parse_server(item) {
            if let Some(ref cset) = country_set {
                if !cset.contains(&parsed.country_code) {
                    continue;
                }
            }
            if let Some(ref flist) = feat_list {
                if !matches_features(&parsed, flist) {
                    continue;
                }
            }
            servers.push(parsed);
        }
    }

    progress_callback(&format!("Ready: Loaded {} servers.", servers.len()));
    Ok(servers)
}
