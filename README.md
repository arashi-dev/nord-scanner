# NordVPN Bulk Connection Benchmark & TUI Tester

A high-performance Terminal User Interface (TUI) for **bulk connection benchmarking of all NordVPN servers simultaneously**, featuring real-time socket probing, millisecond latency measurements, and **persistent SQLite caching with configurable TTL**.

If testing is stopped or restarted, it **automatically detects and passes already-tested servers within the TTL window**, resuming testing only for the remaining pending servers.

---

## 🚀 Features

- **Real Connection Testing**:
  - Connects directly to server listening endpoints (default: TCP port 443 for OpenVPN / SSL / NordWhisper).
  - Performs real 3-way TCP handshake (or optional `--mode tls` for cryptographic SSL handshake), proving actual end-to-end socket connectivity and precise round-trip latency.
- **Simultaneous Bulk Testing**:
  - Powered by Python's `asyncio` and high-capacity concurrency semaphore (e.g. 500–1,000 workers).
  - Benchmark all 7,200+ global NordVPN servers in **under 15–20 seconds** (~400+ tests/second).
- **Persistent Cache with TTL (Resume / Restart Support)**:
  - Backed by an atomic, thread-safe SQLite database (`.cache/nord_test_cache.db`) in WAL mode.
  - Configurable TTL (e.g. `--ttl 6h`, `--ttl 30m`, `--ttl 1d`).
  - **On restart**: Reads cached servers within TTL, marks them as passed, loads their metrics into the dashboard, and tests **only the remaining unverified servers**.
- **Interactive Rich TUI**:
  - **Header & Configuration**: Total servers, cached count, pending count, active concurrency, port, and TTL.
  - **Live Metrics Dashboard**: Real-time progress bar, online/offline count with percentages, Min/Avg/Max latency, scan speed (tests/sec), elapsed time, and ETA.
  - **Live Probe Stream**: Real-time feed of recent server tests showing country, server name, station IP, status, latency color grading, and source (Live vs Cache).
  - **Top 10 Fastest Servers**: Dynamic real-time leaderboard sorted by lowest ping with current server load percentage.
  - **Hotkeys**:
    - `[Q]` / `Ctrl+C`: Gracefully stop and export results.
    - `[P]`: Pause / Resume testing.
    - `[R]`: Retest All (clear cache and force fresh benchmark).
    - `[E]`: Export results to CSV & JSON on demand.
- **Data Exporting**:
  - Automatic and on-demand export to `nord_results.csv` and `nord_results.json` ranked by latency.

---

## 📦 Installation & Requirements

Requires **Python 3.10+**.

Install dependencies:
```bash
pip install -r requirements.txt
```

*(Dependencies: `rich>=13.0.0`, `httpx>=0.24.0`)*

---

## ⚡ Quick Start

### 1. Test All 7,200+ Servers
```bash
python main.py
```

### 2. Test with Custom Concurrency (e.g., 1,000 Simultaneous Workers)
```bash
python main.py --concurrency 1000
```
*(Tip: Setting `--concurrency 0` launches all servers simultaneously).*

### 3. Custom Cache TTL (e.g., 2 hours or 30 minutes)
```bash
# Cache test results for 2 hours (restarting passes servers tested in the last 2 hours)
python main.py --ttl 2h

# Or 30 minutes
python main.py --ttl 30m
```

### 4. Filter by Country
```bash
# Test only US servers (~2,500 servers)
python main.py --country US

# Test multiple countries (e.g. Germany, UK, and Netherlands)
python main.py --country DE,GB,NL
```

### 5. Force Full Retest (Bypass Cache)
```bash
python main.py --force
```

### 6. Full Cryptographic TLS Handshake Mode
```bash
python main.py --mode tls --concurrency 200
```

---

## 🛠️ CLI Options

| Argument | Shorthand | Default | Description |
|---|---|---|---|
| `--concurrency` | `-c` | `500` | Number of simultaneous connections (0 for unlimited). |
| `--port` | `-p` | `443` | Target server port (443 for OpenVPN/SSL/NordWhisper). |
| `--timeout` | `-t` | `3.0` | Socket connection timeout in seconds per server. |
| `--ttl` | | `6h` | Cache TTL (`30m`, `2h`, `1d`, `3600s`). Passed on restart. |
| `--country` | | `all` | Filter by country code (`US`, `DE`, `CA,GB`, or `all`). |
| `--mode` | | `tcp` | `tcp` (fastest 3-way handshake) or `tls` (SSL negotiation). |
| `--force` | | `False` | Retest all servers by clearing existing cache. |
| `--refresh-servers` | | `False` | Re-fetch the server list from the NordVPN API. |
| `--export-csv` | | `nord_results.csv` | Output file path for CSV benchmark report. |
| `--export-json` | | `nord_results.json` | Output file path for JSON benchmark report. |
| `--auto-exit` | | `False` | Automatically exit immediately upon scan completion. |

---

## 📂 Project Architecture

```
nifty-darwin/
├── main.py                     # Main CLI entrypoint
├── requirements.txt            # Python dependencies (rich, httpx)
├── README.md                   # Documentation & usage guide
├── nord_results.csv            # Ranked CSV benchmark export
├── nord_results.json           # Ranked JSON benchmark export
├── .cache/
│   ├── nord_servers_metadata.json   # Cached server catalog from NordVPN API
│   └── nord_test_cache.db           # Persistent SQLite database storing test results
└── nord_tester/
    ├── __init__.py
    ├── models.py               # ServerInfo, TestResult, TestStatus dataclasses
    ├── api.py                  # NordVPN API client with local metadata cache
    ├── cache.py                # SQLite cache manager with TTL logic & WAL mode
    ├── keyboard.py             # Cross-platform non-blocking key listener
    ├── scanner.py              # High-concurrency async socket probe engine
    ├── tui.py                  # Rich-based Terminal User Interface renderer
    ├── exporter.py             # CSV and JSON report exporter
    └── runner.py               # Main coordinator & reactive lifecycle loop
```

---

## 📊 Benchmark Output Example

```
=== NordVPN Test Run Complete ===
Total Servers:   7,205
Passed (Cached): 7,205
Online:          7,200
Offline/Timeout: 5
Latency:         Min: 101.0ms | Avg: 1116.3ms | Max: 2772.7ms
CSV Export:      c:\...\nord_results.csv
JSON Export:     c:\...\nord_results.json
```
