use clap::Parser;
use nord_scanner::runner::{parse_ttl, AppRunner};

#[derive(Parser, Debug)]
#[command(
    name = "nord-scanner",
    about = "Nord Scanner - Real-time concurrent socket benchmark with persistent caching.",
    version = "0.1.0"
)]
struct Args {
    /// Number of concurrent connection tests (default: 500, set to 0 for unlimited/all simultaneous).
    #[arg(short = 'c', long = "concurrency", default_value_t = 500)]
    concurrency: usize,

    /// Port to test on each NordVPN server (default: 443 for OpenVPN/SSL/NordWhisper).
    #[arg(short = 'p', long = "port", default_value_t = 443)]
    port: u16,

    /// Connection timeout in seconds per server.
    #[arg(short = 't', long = "timeout", default_value_t = 30.0)]
    timeout: f64,

    /// Cache Time-To-Live duration. Formats: 30m, 6h, 1d, 3600s.
    #[arg(long = "ttl", default_value = "6h")]
    ttl: String,

    /// Filter servers by country code (e.g. US, DE, CA,GB or 'all').
    #[arg(long = "country", default_value = "all")]
    country: String,

    /// Filter servers by protocol/feature (e.g. 'obfuscated', 'nordwhisper', 'obfuscated,nordwhisper', 'p2p').
    #[arg(long = "type", visible_alias = "feature", default_value = "")]
    feature: String,

    /// Filter to Obfuscated / XOR servers only.
    #[arg(long = "obfuscated")]
    obfuscated: bool,

    /// Filter to NordWhisper servers only.
    #[arg(long = "nordwhisper")]
    nordwhisper: bool,

    /// Connection test mode: 'tcp' (fastest 3-way handshake) or 'tls' (full cryptographic SSL handshake).
    #[arg(long = "mode", default_value = "tcp")]
    mode: String,

    /// Force full benchmark bypassing cache.
    #[arg(long = "force", default_value_t = false)]
    force: bool,

    /// Re-fetch the entire server list from NordVPN API instead of using cached metadata.
    #[arg(long = "refresh-servers", default_value_t = false)]
    refresh_servers: bool,

    /// Export benchmark results to CSV file.
    #[arg(long = "export-csv", default_value = "nord_results.csv")]
    export_csv: String,

    /// Export benchmark results to JSON file.
    #[arg(long = "export-json", default_value = "nord_results.json")]
    export_json: String,

    /// Automatically export results and exit once all servers are benchmarked.
    #[arg(long = "auto-exit", default_value_t = false)]
    auto_exit: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let ttl_seconds = parse_ttl(&args.ttl);

    let country_filter = if args.country.trim().is_empty() || args.country.eq_ignore_ascii_case("all") {
        None
    } else {
        Some(
            args.country
                .split(',')
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect(),
        )
    };

    let mut feature_filter: Vec<String> = Vec::new();
    if !args.feature.trim().is_empty() {
        for f in args.feature.split(',') {
            let f = f.trim();
            if !f.is_empty() {
                feature_filter.push(f.to_string());
            }
        }
    }
    if args.obfuscated && !feature_filter.iter().any(|f| f == "obfuscated") {
        feature_filter.push("obfuscated".to_string());
    }
    if args.nordwhisper && !feature_filter.iter().any(|f| f == "nordwhisper") {
        feature_filter.push("nordwhisper".to_string());
    }

    let feature_filter = if feature_filter.is_empty() {
        None
    } else {
        Some(feature_filter)
    };

    let runner = AppRunner {
        concurrency: if args.concurrency == 0 {
            10000
        } else {
            args.concurrency
        },
        port: args.port,
        timeout: args.timeout,
        ttl_str: args.ttl,
        ttl_seconds,
        force_retest: args.force,
        refresh_servers: args.refresh_servers,
        mode: args.mode,
        country_filter,
        feature_filter,
        csv_export: Some(args.export_csv),
        json_export: Some(args.export_json),
        auto_exit: args.auto_exit,
    };

    runner.run().await
}
