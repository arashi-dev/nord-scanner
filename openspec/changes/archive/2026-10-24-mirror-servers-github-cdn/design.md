# Design

## Context

Currently, `src-tauri/src/api.rs` connects solely to `https://api.nordvpn.com/v1/servers?limit=0` with a 45-second timeout. In regions where NordVPN domains are blocked by censorship or DNS tampering, this call fails, preventing initial server discovery on fresh installations. The raw API response is ~25.7 MB, containing extensive metadata not used by the scanner.

See [proposal.md](proposal.md) for background and motivation.

## Goals / Non-Goals

**Goals:**
- Automate scheduled retrieval, filtering, and compression of NordVPN server metadata in GitHub Actions.
- Distribute the catalog across multiple free CDNs (GitHub Releases, jsDelivr, Statically, and GitHub Raw) without repository size bloat.
- Implement an automated client-side fallback cascade in Rust (`src-tauri/src/api.rs`) with short per-endpoint timeouts (4–5s) and transparent in-memory decompression.
- Preserve full backwards compatibility with existing frontend IPC calls and local cache structures.

**Non-Goals:**
- Real-time proxying (GitHub Actions does not act as a live reverse-proxy tunnel).
- Adding in-app SOCKS5/HTTP proxy settings (explicitly rejected during user exploration).
- Modifying the underlying `ServerInfo` schema or SQLite diagnostic tables.

## Decisions

### Decision 1: Scheduled ETL in GitHub Actions with Configurable Interval
- **Rationale**: Public GitHub repositories provide unlimited GitHub Actions runner minutes. The server catalog only changes periodically, making a batch sync pipeline ideal.
- **Implementation**: The workflow triggers on schedule (default every 6 hours) or manual `workflow_dispatch`. It reads the `SYNC_INTERVAL_HOURS` repository variable, exiting immediately if the desired interval has not elapsed, allowing maintainers to tune frequency from repository settings without committing code.
- **Alternatives Considered**:
  - *Cloudflare Workers dynamic proxy*: Requires managing worker tokens and custom domains to avoid `*.workers.dev` blocks.
  - *Hardcoding cron expressions*: Requires git commits for every schedule tweak.

### Decision 2: Payload Stripping and In-Memory Compression
- **Rationale**: The raw API returns 25.7 MB of data, whereas Nord Scanner only requires `id`, `name`, `hostname`, `station`, `country_name`, `country_code`, `city`, `load`, `technologies`, and `groups`. Stripping unused properties reduces JSON to ~2.3 MB. Applying gzip compression further drops it to ~146 KB (a 99.4% reduction).
- **Implementation**: The workflow script generates both `servers.json` (uncompressed minified) and `servers.json.gz` (gzipped).
- **Alternatives Considered**:
  - *Distributing raw 25 MB JSON*: Excess bandwidth consumption and slow downloads over throttled networks.

### Decision 3: Zero-Bloat Distribution via Release Assets & Orphan Branch
- **Rationale**: Committing large JSON snapshots directly to Git history would bloat the repository clone size by gigabytes each year.
- **Implementation**:
  - Upload `servers.json` and `servers.json.gz` to a permanent GitHub Release tagged `servers-cache`.
  - Force-push `servers.json` and `servers.json.gz` to an isolated orphan branch `servers-data` with a single commit (`git checkout --orphan`, `git push --force`).
  - This immediately makes the file accessible via jsDelivr (`cdn.jsdelivr.net`), Statically (`cdn.statically.io`), and GitHub Raw.
- **Alternatives Considered**:
  - *Standard branch commit history*: Rapidly exceeds GitHub repository size recommendations.

### Decision 4: Client Sequential Fallback Cascade
- **Rationale**: Waiting 45 seconds on a blocked endpoint degrades user experience. Using sequential candidates with 4-5s timeouts allows the client to pivot to an unblocked mirror in seconds.
- **Sequence**:
  1. Direct API (`api.nordvpn.com`) [Timeout: 4s]
  2. GitHub Release Asset (`servers.json.gz`) [Timeout: 5s]
  3. jsDelivr CDN (`servers.json.gz`) [Timeout: 5s]
  4. Statically CDN (`servers.json`) [Timeout: 5s]
  5. GitHub Raw Mirror (`servers.json`) [Timeout: 5s]
  6. Local Cache File (`.cache/nord_servers_metadata.json`)
- **Alternatives Considered**:
  - *Concurrent "race" across all mirrors*: Wastes bandwidth and hits multiple free CDNs simultaneously on every launch.

### Decision 5: Rust In-Memory Decompression
- **Rationale**: Downloading `.json.gz` saves significant bandwidth, but the client must seamlessly unpack it in-memory.
- **Implementation**: Use `flate2::read::GzDecoder` or `reqwest` gzip decoding based on byte inspection (magic bytes `0x1F, 0x8B`) or URL suffix `.gz`.

## Risks / Trade-offs

- **[Risk] GitHub Release or jsDelivr blocked in a specific region**  
  &rarr; *Mitigation*: The client cascades through multiple independent CDN providers (GitHub S3, jsDelivr Fastly/Cloudflare, Statically, Raw GitHub) before falling back to local cache.
- **[Risk] NordVPN API schema changes**  
  &rarr; *Mitigation*: The workflow script performs validation checks on the parsed server count before overwriting release assets. If NordVPN's API returns an error or empty list, the workflow fails without overwriting the previous valid mirror.
- **[Risk] Gzip payload corruption during transit**  
  &rarr; *Mitigation*: The client validates the decompressed JSON structure; if parsing fails, it seamlessly tries the next mirror candidate in the cascade.

## Migration Plan

1. Create `.github/workflows/sync-servers.yml` and test run via `workflow_dispatch`.
2. Verify that the `servers-cache` release tag and `servers-data` orphan branch are created with `servers.json` and `servers.json.gz`.
3. Add `flate2` to `src-tauri/Cargo.toml`.
4. Update `src-tauri/src/api.rs` with candidate mirror definitions, fast timeouts, and decompression logic.
5. Validate both local cache and fallback mirrors via cargo test and application startup.
