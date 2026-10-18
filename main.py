#!/usr/bin/env python3
"""
NordVPN Bulk Connection Tester - Entrypoint
Real-time concurrent socket benchmark and availability monitor with persistent caching.
"""

import argparse
import asyncio
import sys
from typing import List, Optional

if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

from nord_tester.runner import AppRunner


def parse_args():
    parser = argparse.ArgumentParser(
        description="NordVPN Bulk Connection Tester - Real-time concurrent socket benchmark with persistent caching."
    )
    parser.add_argument(
        "-c", "--concurrency",
        type=int,
        default=500,
        help="Number of concurrent connection tests (default: 500, set to 0 for unlimited/all simultaneous).",
    )
    parser.add_argument(
        "-p", "--port",
        type=int,
        default=443,
        help="Port to test on each NordVPN server (default: 443 for OpenVPN/SSL/NordWhisper).",
    )
    parser.add_argument(
        "-t", "--timeout",
        type=float,
        default=3.0,
        help="Connection timeout in seconds per server (default: 3.0s).",
    )
    parser.add_argument(
        "--ttl",
        type=str,
        default="6h",
        help="Cache Time-To-Live duration. Formats: 30m, 6h, 1d, 3600s (default: 6h). Already tested servers within this window are passed/skipped.",
    )
    parser.add_argument(
        "--country",
        type=str,
        default="all",
        help="Filter servers by country code (e.g. US, DE, CA,GB or 'all'). Default: all.",
    )
    parser.add_argument(
        "--mode",
        choices=["tcp", "tls"],
        default="tcp",
        help="Connection test mode: 'tcp' (real 3-way handshake, fastest) or 'tls' (full cryptographic SSL handshake). Default: tcp.",
    )
    parser.add_argument(
        "--force", "--no-cache",
        dest="force",
        action="store_true",
        help="Force retesting of all servers by ignoring and clearing cached results.",
    )
    parser.add_argument(
        "--refresh-servers",
        action="store_true",
        help="Force refetching the complete server catalog from NordVPN API instead of using local server metadata cache.",
    )
    parser.add_argument(
        "--export-csv",
        type=str,
        default="nord_results.csv",
        help="Path for exporting results to CSV (default: nord_results.csv, set to '' to disable).",
    )
    parser.add_argument(
        "--export-json",
        type=str,
        default="nord_results.json",
        help="Path for exporting results to JSON (default: nord_results.json, set to '' to disable).",
    )
    parser.add_argument(
        "--auto-exit",
        action="store_true",
        help="Automatically exit immediately when all tests finish.",
    )

    return parser.parse_args()


def main():
    args = parse_args()

    # Process country filter
    country_filter: Optional[List[str]] = None
    if args.country and args.country.strip().lower() != "all":
        country_filter = [c.strip().upper() for c in args.country.split(",") if c.strip()]

    # Convert 0 concurrency to very high number so all are tested simultaneously
    concurrency = 10000 if args.concurrency <= 0 else args.concurrency

    runner = AppRunner(
        concurrency=concurrency,
        port=args.port,
        timeout=args.timeout,
        ttl_str=args.ttl,
        force_retest=args.force,
        refresh_servers=args.refresh_servers,
        mode=args.mode,
        country_filter=country_filter,
        csv_export=args.export_csv if args.export_csv else None,
        json_export=args.export_json if args.export_json else None,
        auto_exit=args.auto_exit,
    )

    try:
        asyncio.run(runner.run())
    except KeyboardInterrupt:
        print("\nInterrupted by user. Exiting cleanly...")
        sys.exit(0)


if __name__ == "__main__":
    main()
