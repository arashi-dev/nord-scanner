# Tasks

## 1. Backend (Rust / Tauri)

- [x] 1.1 Add `flate2 = "1.0"` to `src-tauri/Cargo.toml` and verify `cargo check --package nord-scanner` compiles successfully.
- [x] 1.2 Implement candidate endpoint configuration in `src-tauri/src/api.rs` (Direct API with 4s timeout, GitHub Release, jsDelivr CDN, Statically, GitHub Raw).
- [x] 1.3 Implement sequential fallback loop and in-memory gzip decompression in `src-tauri/src/api.rs` to transparently handle both `.json.gz` and uncompressed `.json` payloads.
- [x] 1.4 Update `refresh_servers_data` and `fetch_all_servers` to use the fallback cascade and verify `cargo check` passes without warnings.

## 2. CI/CD & Mirror Pipeline (GitHub Actions)

- [x] 2.1 Create `.github/workflows/sync-servers.yml` with scheduled execution, manual `workflow_dispatch`, and configurable `SYNC_INTERVAL_HOURS` variable check.
- [x] 2.2 Implement data minimization script in the workflow that fetches `https://api.nordvpn.com/v1/servers?limit=0`, strips unused fields into `servers.json`, and compresses to `servers.json.gz`.
- [x] 2.3 Implement workflow publish steps to update the `servers-cache` GitHub Release tag and force-push to the orphan branch `servers-data`.

## 3. Frontend & UX

- [x] 3.1 Verify `Header.tsx` and `App.tsx` error handling when fallback mirrors are utilized and ensure `npm run build` succeeds without type errors.

## 4. Testing & Validation

- [x] 4.1 Add automated test in `src-tauri/tests/test_all.rs` validating decompression and parsing of minified `ServerInfo` catalog payloads.
- [x] 4.2 Execute `cargo test --package nord-scanner` and verify all tests pass.
- [x] 4.3 Run full build validation via `npm run build` and `cargo check --package nord-scanner`.
