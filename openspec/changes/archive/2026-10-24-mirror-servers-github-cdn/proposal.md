# Proposal

## Why

In several regions and restricted networks (such as Iran, China, and enterprise corporate environments), `api.nordvpn.com` is actively blocked or throttled at the DNS and TLS/SNI level. When users launch Nord Scanner or initiate a server refresh on these networks, the client currently waits up to 45 seconds before timing out, leaving new installations without cached data completely unusable.

By publishing a lightweight, minified, and gzipped server catalog snapshot via a scheduled GitHub Actions workflow (running on unblocked cloud runners) and distributing it across multiple free CDNs (GitHub Releases, jsDelivr, Statically, and GitHub Raw), clients can reliably fetch the server catalog in milliseconds through an automated fallback cascade even when the direct API is blocked.

## What Changes

- **Automated Mirror Pipeline (GitHub Actions)**:
  - Add `.github/workflows/sync-servers.yml` triggered on schedule (default every 6 hours, configurable via `SYNC_INTERVAL_HOURS` variable) and on-demand via `workflow_dispatch`.
  - Fetch raw server data from `https://api.nordvpn.com/v1/servers?limit=0`, strip unused fields (reducing payload from ~25.7 MB to ~2.3 MB JSON), and compress to `.json.gz` (~146 KB).
  - Publish assets to a persistent GitHub Release tagged `servers-cache` and force-push to an orphan branch `servers-data` (zero git repository bloat).
- **Backend Multi-Tier Fallback Cascade (`src-tauri/src/api.rs`)**:
  - Replace single-endpoint fetch with a prioritized candidate cascade:
    1. Direct NordVPN API (`api.nordvpn.com`) with a fast 4s timeout.
    2. GitHub Releases Asset (`servers.json.gz`).
    3. jsDelivr CDN (`cdn.jsdelivr.net`).
    4. Statically CDN (`cdn.statically.io`).
    5. Raw GitHub (`raw.githubusercontent.com`).
    6. Local disk cache (`.cache/nord_servers_metadata.json`).
  - Transparent in-memory decompression for `.json.gz` payloads.
  - Retain existing `ServerInfo` schema and SQLite persistence without breaking changes.
- **Frontend / UX Feedback**:
  - Keep existing IPC commands (`load_servers`, `refresh_servers`) contract intact.
  - Surface active data source / mirror status in progress logs or error toasts if fallback occurs.

## Capabilities

### New Capabilities
- `server-catalog-mirror`: Multi-tier fallback synchronization for NordVPN server catalog metadata via GitHub Actions pipeline and global CDN distribution.

### Modified Capabilities
<!-- No existing capabilities to modify -->

## Impact

- **Rust Backend**:
  - `src-tauri/src/api.rs`: Refactor fetch logic to iterate through candidate mirror URLs with fast timeouts and gzip decompression support.
  - `src-tauri/Cargo.toml`: Add `flate2 = "1.0"` (or reqwest `gzip` feature) for fast in-memory decompression.
- **CI/CD & Workflows**:
  - New workflow file `.github/workflows/sync-servers.yml` with permissions `contents: write`.
  - Zero billing impact: runs within free unlimited public repo GitHub Actions minutes.
- **Cross-Platform Implications**:
  - Pure HTTPS network layer: works identically across Windows, macOS, Linux, and Android.
- **Breaking Changes**:
  - None. Tauri IPC commands (`refresh_servers`, `load_servers`), response types, and local SQLite schemas remain 100% backwards-compatible.
