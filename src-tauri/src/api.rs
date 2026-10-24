use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::models::ServerInfo;

pub const NORD_SERVERS_URL: &str = "https://api.nordvpn.com/v1/servers?limit=0";
pub const META_CACHE_TTL_SECS: u64 = 24 * 3600;

#[derive(Debug, Clone)]
pub struct ServerMirror {
    pub name: &'static str,
    pub url: &'static str,
    pub timeout: Duration,
}

pub const SERVER_MIRRORS: &[ServerMirror] = &[
    ServerMirror {
        name: "Direct NordVPN API",
        url: "https://api.nordvpn.com/v1/servers?limit=0",
        timeout: Duration::from_secs(4),
    },
    ServerMirror {
        name: "GitHub Releases CDN",
        url: "https://github.com/arashi-dev/nord-scanner/releases/download/servers-cache/servers.json.gz",
        timeout: Duration::from_secs(5),
    },
    ServerMirror {
        name: "jsDelivr CDN",
        url: "https://cdn.jsdelivr.net/gh/arashi-dev/nord-scanner@servers-data/servers.json.gz",
        timeout: Duration::from_secs(5),
    },
    ServerMirror {
        name: "Statically CDN",
        url: "https://cdn.statically.io/gh/arashi-dev/nord-scanner/servers-data/servers.json",
        timeout: Duration::from_secs(5),
    },
    ServerMirror {
        name: "GitHub Raw Mirror",
        url: "https://raw.githubusercontent.com/arashi-dev/nord-scanner/servers-data/servers.json",
        timeout: Duration::from_secs(5),
    },
];

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

pub fn decompress_if_gzipped(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = GzDecoder::new(bytes);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .context("Failed to decompress gzip server catalog stream")?;
        Ok(decompressed)
    } else {
        Ok(bytes.to_vec())
    }
}

pub fn parse_servers_bytes(bytes: &[u8]) -> Result<Vec<ServerInfo>> {
    let decompressed = decompress_if_gzipped(bytes)?;

    // 1. Try parsing directly as minified Vec<ServerInfo>
    if let Ok(servers) = serde_json::from_slice::<Vec<ServerInfo>>(&decompressed) {
        if !servers.is_empty() {
            return Ok(servers);
        }
    }

    // 2. Try parsing as raw NordVPN API response Vec<RawServer>
    let raw_list: Vec<RawServer> = serde_json::from_slice(&decompressed)
        .context("Failed to deserialize NordVPN server catalog JSON (tried ServerInfo and RawServer)")?;

    let servers: Vec<ServerInfo> = raw_list.into_iter().filter_map(parse_server).collect();
    Ok(servers)
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

    // Try reading count with dual support (ServerInfo and RawServer)
    let count = if let Ok(data) = fs::read(&cache_file) {
        if let Ok(servers) = parse_servers_bytes(&data) {
            servers.len()
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

pub async fn fetch_servers_with_fallback<F>(
    custom_dir: Option<&Path>,
    progress_callback: Option<&F>,
) -> Result<(Vec<ServerInfo>, u64)>
where
    F: Fn(&str),
{
    let cache_file = get_cache_file(custom_dir);
    let mut last_error: Option<String> = None;

    for mirror in SERVER_MIRRORS {
        if let Some(cb) = progress_callback {
            cb(&format!("Connecting to {}...", mirror.name));
        }

        let client = match reqwest::Client::builder()
            .timeout(mirror.timeout)
            .user_agent("NordScanner/0.1")
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                last_error = Some(format!("Failed to build HTTP client for {}: {}", mirror.name, e));
                continue;
            }
        };

        match client.get(mirror.url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let err_msg = format!("{} responded with HTTP {}", mirror.name, resp.status());
                    if let Some(cb) = progress_callback {
                        cb(&format!("{} - trying next mirror...", err_msg));
                    }
                    last_error = Some(err_msg);
                    continue;
                }

                match resp.bytes().await {
                    Ok(raw_bytes) => {
                        match parse_servers_bytes(&raw_bytes) {
                            Ok(servers) if !servers.is_empty() => {
                                // Save cleaned server catalog to cache as compact JSON
                                if let Ok(json_bytes) = serde_json::to_vec(&servers) {
                                    let _ = fs::write(&cache_file, json_bytes);
                                }
                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();

                                if let Some(cb) = progress_callback {
                                    cb(&format!("Loaded {} servers via {}.", servers.len(), mirror.name));
                                }
                                return Ok((servers, now));
                            }
                            Ok(_) => {
                                last_error = Some(format!("{} returned empty server list", mirror.name));
                            }
                            Err(e) => {
                                last_error = Some(format!("Failed to parse response from {}: {:#}", mirror.name, e));
                            }
                        }
                    }
                    Err(e) => {
                        last_error = Some(format!("Failed to read response body from {}: {:#}", mirror.name, e));
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Connection to {} failed: {}", mirror.name, e);
                if let Some(cb) = progress_callback {
                    cb(&format!("{} - trying next mirror...", err_msg));
                }
                last_error = Some(err_msg);
            }
        }
    }

    // If all remote mirrors failed, attempt loading from local cache
    if cache_file.exists() {
        if let Ok(servers) = load_cached_servers_data(custom_dir) {
            if !servers.is_empty() {
                if let Some(cb) = progress_callback {
                    cb(&format!(
                        "All mirrors failed. Loaded {} servers from local cache.",
                        servers.len()
                    ));
                }
                let mtime = fs::metadata(&cache_file)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()))
                    .unwrap_or_default();
                return Ok((servers, mtime));
            }
        }
    }

    Err(anyhow::anyhow!(
        "Failed to fetch servers from any mirror, and no valid local cache was found. Last error: {}",
        last_error.unwrap_or_else(|| "Unknown network error".to_string())
    ))
}

pub async fn refresh_servers_data(custom_dir: Option<&Path>) -> Result<(Vec<ServerInfo>, u64)> {
    fetch_servers_with_fallback::<fn(&str)>(custom_dir, None).await
}

pub fn load_cached_servers_data(custom_dir: Option<&Path>) -> Result<Vec<ServerInfo>> {
    let cache_file = get_cache_file(custom_dir);
    let bytes = fs::read(&cache_file).context("No cached servers file found")?;
    parse_servers_bytes(&bytes)
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
    let mut servers_data: Option<Vec<ServerInfo>> = None;

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
                        if let Ok(cached) = load_cached_servers_data(None) {
                            if !cached.is_empty() {
                                servers_data = Some(cached);
                            }
                        }
                    }
                }
            }
        }
    }

    if servers_data.is_none() {
        let (fetched_servers, _) = fetch_servers_with_fallback(None, Some(&progress_callback)).await?;
        servers_data = Some(fetched_servers);
    }

    let raw_list = servers_data.context("No server metadata found")?;

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
        if let Some(ref cset) = country_set {
            if !cset.contains(&item.country_code) {
                continue;
            }
        }
        if let Some(ref flist) = feat_list {
            if !matches_features(&item, flist) {
                continue;
            }
        }
        servers.push(item);
    }

    progress_callback(&format!("Ready: Loaded {} servers.", servers.len()));
    Ok(servers)
}
