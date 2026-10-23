# Nord Scanner (Tauri v2 Desktop & Mobile)

A high-performance GUI and engine for **bulk connection benchmarking and scanning of all NordVPN servers simultaneously**, built with **Tauri v2, React 19, Vite, Tailwind CSS, and Shadcn UI**. Features real-time socket probing, millisecond latency measurements, persistent SQLite caching, and multiplatform support across Windows, macOS, Linux, and Android.

---

## 🚀 Features

- **Tauri v2 Multiplatform Desktop & Mobile App**:
  - Supported platforms: **Windows**, **macOS**, **Linux**, and **Android**.
  - **Single Instance Support**: Enforces a single application instance on desktop and brings the active window to focus when re-launched.
  - **Auto-Updater Support**: Built-in Tauri v2 updater (`@tauri-apps/plugin-updater`) for seamless over-the-air updates.
- **Bulk Real Connection Testing**:
  - Connects directly to server listening endpoints (default: TCP port 443 for OpenVPN / SSL / NordWhisper, or optional TLS cryptographic handshake).
  - High-capacity Tokio async concurrency (tested up to 1,000+ workers).
  - Benchmark all 7,200+ global NordVPN servers in **under 15–20 seconds** (~400+ tests/second).
- **Persistent Filters & Settings with Reset**:
  - Filter by Country (with automatic counts), Port, Protocol / Feature (`obfuscated`, `nordwhisper`, `p2p`, `double`, `onion`, `dedicated`), Mode (`tcp` vs `tls`), Concurrency, Search keyword, and Status.
  - **Default Timeout**: Configured to **30 seconds** by default.
  - All filter settings persist in `localStorage` across restarts.
  - **Reset Button**: One-click reset restoring all defaults (including 30s timeout, port 443, etc.).
- **Decoupled Manual Server Refresh**:
  - Re-fetching the complete server catalog from the NordVPN API is done **manually and separately from scanning**.
  - Dynamically displays the **last time it was refreshed successfully in time-ago format** (e.g. `just now`, `5 minutes ago`, `3 hours ago`).
- **Live Metrics & Leaderboard**:
  - Real-time progress bar, online/offline counts, tests/sec scan speed, elapsed time, and ETA.
  - Dynamic Top 10 fastest online servers ranked by lowest latency with one-click IP copy.
- **Automated GitHub Releases Workflow**:
  - Automated CI/CD on tag push (`v*`) in [`.github/workflows/release.yml`](.github/workflows/release.yml).
  - Builds Windows installers (`.msi`, `.exe`), macOS bundles (`.dmg`, `.app`), Linux packages (`.deb`, `.AppImage`), and Android APKs (`.apk`).
  - Publishes assets and signed updater bundles to GitHub Releases.

---

## 🛠️ Tech Stack

- **Framework**: Tauri v2 (`@tauri-apps/api@2`, `tauri@2`)
- **Frontend**: React 19, TypeScript, Vite 6
- **Styling**: Tailwind CSS, Shadcn UI architecture, Lucide Icons
- **Backend / Core Engine**: Rust (Tokio, reqwest, rusqlite with bundled SQLite, rustls)
- **Plugins**: `tauri-plugin-single-instance`, `tauri-plugin-updater`

---

## 📦 Getting Started

### Prerequisites

- **Node.js**: v20+ or v22+
- **Rust**: 1.75+ (`rustc` and `cargo`)
- For Android: Android Studio / Android SDK & NDK

### Installation

```bash
npm install
```

### Development Mode

Run the desktop application in dev mode with live reload:

```bash
npm run tauri dev
```

Or run the web frontend in the browser:

```bash
npm run dev
```

### Production Build

Build the optimized desktop application for your current operating system:

```bash
npm run tauri build
```

The installer and executable will be generated in `src-tauri/target/release/bundle/`.

---

## 📱 Android Build

The Android target is configured in `src-tauri/gen/android`.

To run or build on Android:

```bash
# Development mode on connected Android device/emulator
npm run tauri android dev

# Production APK build
npm run tauri android build -- --apk
```

---

## 🚀 GitHub Actions Release Workflow

Pushing a git tag matching `v*` triggers the automated release workflow:

```bash
git tag v1.0.0
git push origin v1.0.0
```

This workflow builds and publishes release assets for:
- **Windows**: `.msi` and `.exe` (NSIS)
- **macOS**: `.dmg` and `.app`
- **Linux**: `.deb` and `.AppImage`
- **Android**: `.apk` (Universal, ARM64, ARMv7, x86_64)
- **Auto-Updater JSON**: `latest.json` with Minisign signatures
