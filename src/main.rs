mod api;
mod cache;
mod exporter;
mod models;
mod runner;
mod scanner;
mod tui;

use clap::Parser;
use runner::{parse_ttl, AppRunner};

#[derive(Parser, Debug)]
#[command(
    name = "nord_tester",
    about = "NordVPN Bulk Connection Tester - Real-time concurrent socket benchmark with persistent caching.",
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
    #[arg(short = 't', long = "timeout", default_value_t = 3.0)]
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

    /// Connection test mode: 'tcp' (real 3-way handshake) or 'tls' (full cryptographic SSL handshake).
    #[arg(long = "mode", default_value = "tcp")]
    mode: String,

    /// Force retesting of all servers by ignoring and clearing cached results.
    #[arg(long = "force", visible_alias = "no-cache")]
    force: bool,

    /// Force refetching the complete server catalog from NordVPN API instead of using local server metadata cache.
    #[arg(long = "refresh-servers")]
    refresh_servers: bool,

    /// Path for exporting results to CSV (set to '' to disable).
    #[arg(long = "export-csv", default_value = "nord_results.csv")]
    export_csv: String,

    /// Path for exporting results to JSON (set to '' to disable).
    #[arg(long = "export-json", default_value = "nord_results.json")]
    export_json: String,

    /// Automatically exit immediately when all tests finish.
    #[arg(long = "auto-exit")]
    auto_exit: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Process country filter
    let country_filter: Option<Vec<String>> = if !args.country.is_empty()
        && !args.country.trim().eq_ignore_ascii_case("all")
    {
        Some(
            args.country
                .split(',')
                .map(|c| c.trim().to_uppercase())
                .filter(|c| !c.is_empty())
                .collect(),
        )
    } else {
        None
    };

    // Process feature/type filter
    let mut feature_filter: Vec<String> = Vec::new();
    if !args.feature.is_empty() {
        for f in args.feature.split(',') {
            let tr = f.trim().to_lowercase();
            if !tr.is_empty() {
                feature_filter.push(tr);
            }
        }
    }
    if args.obfuscated && !feature_filter.iter().any(|f| f == "obfuscated") {
        feature_filter.push("obfuscated".to_string());
    }
    if args.nordwhisper && !feature_filter.iter().any(|f| f == "nordwhisper") {
        feature_filter.push("nordwhisper".to_string());
    }
    let feature_filter_opt = if !feature_filter.is_empty() {
        Some(feature_filter)
    } else {
        None
    };

    let concurrency = if args.concurrency == 0 {
        10000
    } else {
        args.concurrency
    };

    let ttl_seconds = parse_ttl(&args.ttl);

    let csv_export = if args.export_csv.trim().is_empty() {
        None
    } else {
        Some(args.export_csv)
    };

    let json_export = if args.export_json.trim().is_empty() {
        None
    } else {
        Some(args.export_json)
    };

    let runner = AppRunner {
        concurrency,
        port: args.port,
        timeout: args.timeout,
        ttl_str: args.ttl,
        ttl_seconds,
        force_retest: args.force,
        refresh_servers: args.refresh_servers,
        mode: args.mode,
        country_filter,
        feature_filter: feature_filter_opt,
        csv_export,
        json_export,
        auto_exit: args.auto_exit,
    };

    runner.run().await?;
    Ok(())
}
