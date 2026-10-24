use nord_scanner::cache::CacheManager;
use nord_scanner::exporter::export_results;
use nord_scanner::models::{ServerInfo, TestResult, TestStatus};
use nord_scanner::runner::parse_ttl;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_parse_ttl_values() {
    assert_eq!(parse_ttl("30m"), 1800.0);
    assert_eq!(parse_ttl("6h"), 21600.0);
    assert_eq!(parse_ttl("1d"), 86400.0);
    assert_eq!(parse_ttl("3600s"), 3600.0);
    assert_eq!(parse_ttl("120"), 120.0);
}

#[test]
fn test_server_info_features() {
    let s = ServerInfo {
        id: 1,
        name: "Test Server".to_string(),
        hostname: "test.nordvpn.com".to_string(),
        station: "1.2.3.4".to_string(),
        country_name: "Germany".to_string(),
        country_code: "DE".to_string(),
        city: "Berlin".to_string(),
        load: 25,
        technologies: vec!["openvpn_xor".to_string(), "nordwhisper".to_string()],
        groups: vec!["legacy_p2p".to_string()],
    };

    assert!(s.is_obfuscated());
    assert!(s.is_nordwhisper());
    let tags = s.feature_tags();
    assert!(tags.contains(&"Obfuscated".to_string()));
    assert!(tags.contains(&"NordWhisper".to_string()));
    assert!(tags.contains(&"P2P".to_string()));
}

#[test]
fn test_cache_and_export() {
    let test_dir = PathBuf::from(".cache/test_db");
    let _ = fs::create_dir_all(&test_dir);
    let db_path = test_dir.join("test.db");
    let _ = fs::remove_file(&db_path);

    let cache_mgr = CacheManager::new(Some(&db_path)).expect("Failed to init cache");

    let sample_result = TestResult {
        server_id: 9999,
        name: "Mock #1".to_string(),
        hostname: "mock1.nordvpn.com".to_string(),
        station: "10.0.0.1".to_string(),
        country_name: "France".to_string(),
        country_code: "FR".to_string(),
        city: "Paris".to_string(),
        load: 10,
        port: 443,
        status: TestStatus::Online,
        latency_ms: Some(42.5),
        tested_at: 1700000000.0,
        is_cached: false,
        features: vec!["Standard".to_string()],
        error_msg: None,
    };

    cache_mgr
        .save_batch(&[sample_result.clone()])
        .expect("Save failed");

    // Fresh cache with high TTL
    let fresh = cache_mgr.get_fresh_cache(9999999999.0).expect("Fetch failed");
    assert!(fresh.contains_key(&9999));
    let fetched = &fresh[&9999];
    assert_eq!(fetched.name, "Mock #1");
    assert_eq!(fetched.latency_ms, Some(42.5));

    // Test exporter
    let csv_file = test_dir.join("export.csv");
    let json_file = test_dir.join("export.json");
    let paths = export_results(&[sample_result], Some(&csv_file), Some(&json_file))
        .expect("Export failed");

    assert!(paths.contains_key("csv"));
    assert!(paths.contains_key("json"));
    assert!(csv_file.exists());
    assert!(json_file.exists());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_server_mirrors_order() {
    use nord_scanner::api::SERVER_MIRRORS;
    assert_eq!(SERVER_MIRRORS.len(), 5);
    assert_eq!(SERVER_MIRRORS[0].name, "Direct NordVPN API");
    assert_eq!(SERVER_MIRRORS[1].name, "GitHub Releases CDN");
    assert_eq!(SERVER_MIRRORS[2].name, "jsDelivr CDN");
    assert_eq!(SERVER_MIRRORS[3].name, "Statically CDN");
    assert_eq!(SERVER_MIRRORS[4].name, "GitHub Raw Mirror");
}

#[test]
fn test_decompress_and_parse_minified_servers() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use nord_scanner::api::{decompress_if_gzipped, parse_servers_bytes};
    use std::io::Write;

    let original = vec![
        ServerInfo {
            id: 101,
            name: "US #101".to_string(),
            hostname: "us101.nordvpn.com".to_string(),
            station: "192.168.1.1".to_string(),
            country_name: "United States".to_string(),
            country_code: "US".to_string(),
            city: "New York".to_string(),
            load: 15,
            technologies: vec!["wireguard".to_string()],
            groups: vec!["p2p".to_string()],
        },
        ServerInfo {
            id: 102,
            name: "DE #102".to_string(),
            hostname: "de102.nordvpn.com".to_string(),
            station: "192.168.1.2".to_string(),
            country_name: "Germany".to_string(),
            country_code: "DE".to_string(),
            city: "Frankfurt".to_string(),
            load: 40,
            technologies: vec!["openvpn_xor".to_string()],
            groups: vec![],
        },
    ];

    let json_bytes = serde_json::to_vec(&original).expect("Serialize failed");

    // Test direct parsing of uncompressed JSON
    let parsed_plain = parse_servers_bytes(&json_bytes).expect("Parse plain failed");
    assert_eq!(parsed_plain.len(), 2);
    assert_eq!(parsed_plain[0].hostname, "us101.nordvpn.com");
    assert_eq!(parsed_plain[1].country_code, "DE");

    // Test parsing of gzipped payload
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&json_bytes).expect("Gzip write failed");
    let gzipped_bytes = encoder.finish().expect("Gzip finish failed");

    // Verify gzip magic bytes (0x1f, 0x8b)
    assert!(gzipped_bytes.len() >= 2);
    assert_eq!(gzipped_bytes[0], 0x1f);
    assert_eq!(gzipped_bytes[1], 0x8b);

    // Test decompress_if_gzipped
    let decompressed = decompress_if_gzipped(&gzipped_bytes).expect("Decompress failed");
    assert_eq!(decompressed, json_bytes);

    // Test parse_servers_bytes on gzipped stream directly
    let parsed_gz = parse_servers_bytes(&gzipped_bytes).expect("Parse gzipped failed");
    assert_eq!(parsed_gz.len(), 2);
    assert_eq!(parsed_gz[0].name, "US #101");
    assert_eq!(parsed_gz[1].city, "Frankfurt");
    assert!(parsed_gz[1].is_obfuscated());
}
