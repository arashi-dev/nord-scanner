"""Export test results to CSV and JSON formats."""

import csv
import json
import time
from pathlib import Path
from typing import Dict, List, Optional

from .models import TestResult, TestStatus


def export_results(
    results: List[TestResult],
    csv_path: Optional[Path] = None,
    json_path: Optional[Path] = None,
) -> Dict[str, str]:
    """
    Export results list sorted by latency (fastest first).
    Returns mapping of format -> file path.
    """
    # Sort results: ONLINE with valid latency first (ascending), then others
    sorted_results = sorted(
        results,
        key=lambda r: (
            0 if r.status == TestStatus.ONLINE and r.latency_ms is not None else 1,
            r.latency_ms if r.latency_ms is not None else float("inf"),
        ),
    )

    out_paths = {}

    if csv_path:
        csv_path = Path(csv_path)
        csv_path.parent.mkdir(parents=True, exist_ok=True)
        with open(csv_path, "w", newline="", encoding="utf-8") as f:
            writer = csv.writer(f)
            writer.writerow([
                "Rank",
                "Server ID",
                "Name",
                "Hostname",
                "IP Address",
                "Port",
                "Country Code",
                "Country",
                "City",
                "Features",
                "Status",
                "Latency (ms)",
                "Load (%)",
                "Is Cached",
                "Tested At",
                "Error",
            ])
            for i, r in enumerate(sorted_results, start=1):
                writer.writerow([
                    i if r.status == TestStatus.ONLINE else "-",
                    r.server_id,
                    r.name,
                    r.hostname,
                    r.station,
                    r.port,
                    r.country_code,
                    r.country_name,
                    r.city,
                    ", ".join(r.features) if r.features else "Standard",
                    r.status.value,
                    f"{r.latency_ms:.2f}" if r.latency_ms is not None else "",
                    r.load,
                    "Yes" if r.is_cached else "No",
                    time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(r.tested_at)),
                    r.error_msg or "",
                ])
        out_paths["csv"] = str(csv_path.resolve())

    if json_path:
        json_path = Path(json_path)
        json_path.parent.mkdir(parents=True, exist_ok=True)
        online_count = sum(1 for r in sorted_results if r.status == TestStatus.ONLINE)
        latencies = [r.latency_ms for r in sorted_results if r.latency_ms is not None]
        avg_lat = round(sum(latencies) / len(latencies), 2) if latencies else None

        data = {
            "metadata": {
                "exported_at": time.strftime("%Y-%m-%d %H:%M:%S"),
                "total_servers": len(sorted_results),
                "online_servers": online_count,
                "offline_servers": len(sorted_results) - online_count,
                "avg_latency_ms": avg_lat,
            },
            "servers": [
                {
                    "rank": i if r.status == TestStatus.ONLINE else None,
                    "id": r.server_id,
                    "name": r.name,
                    "hostname": r.hostname,
                    "ip": r.station,
                    "port": r.port,
                    "country": r.country_name,
                    "country_code": r.country_code,
                    "city": r.city,
                    "features": r.features,
                    "status": r.status.value,
                    "latency_ms": r.latency_ms,
                    "load": r.load,
                    "is_cached": r.is_cached,
                    "tested_at": r.tested_at,
                    "error": r.error_msg,
                }
                for i, r in enumerate(sorted_results, start=1)
            ],
        }
        with open(json_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
        out_paths["json"] = str(json_path.resolve())

    return out_paths
