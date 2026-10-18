"""SQLite persistent cache for NordVPN connection test results with TTL."""

import sqlite3
import time
from pathlib import Path
from typing import Dict, List, Optional

from .models import TestResult, TestStatus

DEFAULT_CACHE_DIR = Path(".cache")
DEFAULT_DB_PATH = DEFAULT_CACHE_DIR / "nord_test_cache.db"


class CacheManager:
    """Manages persistent SQLite storage of test results with TTL verification."""

    def __init__(self, db_path: Optional[Path] = None):
        self.db_path = db_path or DEFAULT_DB_PATH
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._init_db()

    def _get_connection(self) -> sqlite3.Connection:
        conn = sqlite3.connect(str(self.db_path), timeout=10.0)
        conn.row_factory = sqlite3.Row
        # Enable WAL mode for high concurrent throughput
        conn.execute("PRAGMA journal_mode=WAL;")
        conn.execute("PRAGMA synchronous=NORMAL;")
        return conn

    def _init_db(self):
        with self._get_connection() as conn:
            conn.execute(
                """
                CREATE TABLE IF NOT EXISTS test_cache (
                    server_id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    hostname TEXT NOT NULL,
                    station TEXT NOT NULL,
                    country_name TEXT,
                    country_code TEXT,
                    city TEXT,
                    load INTEGER,
                    port INTEGER NOT NULL,
                    status TEXT NOT NULL,
                    latency_ms REAL,
                    tested_at REAL NOT NULL,
                    error_msg TEXT
                );
                """
            )
            conn.execute("CREATE INDEX IF NOT EXISTS idx_tested_at ON test_cache(tested_at);")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_status ON test_cache(status);")
            conn.execute("CREATE INDEX IF NOT EXISTS idx_country ON test_cache(country_code);")
            conn.commit()

    def get_fresh_cache(self, ttl_seconds: float) -> Dict[int, TestResult]:
        """
        Return a mapping of server_id -> TestResult for all servers
        tested within the last ttl_seconds.
        """
        min_timestamp = time.time() - ttl_seconds
        results: Dict[int, TestResult] = {}
        with self._get_connection() as conn:
            cursor = conn.execute(
                """
                SELECT * FROM test_cache
                WHERE tested_at >= ?
                """,
                (min_timestamp,),
            )
            for row in cursor.fetchall():
                try:
                    status = TestStatus(row["status"])
                except Exception:
                    status = TestStatus.ERROR

                results[row["server_id"]] = TestResult(
                    server_id=row["server_id"],
                    name=row["name"],
                    hostname=row["hostname"],
                    station=row["station"],
                    country_name=row["country_name"] or "",
                    country_code=row["country_code"] or "",
                    city=row["city"] or "",
                    load=row["load"] or 0,
                    port=row["port"],
                    status=status,
                    latency_ms=row["latency_ms"],
                    tested_at=row["tested_at"],
                    is_cached=True,
                    error_msg=row["error_msg"],
                )
        return results

    def save_batch(self, batch: List[TestResult]):
        """Save a batch of TestResult objects to SQLite."""
        if not batch:
            return
        records = [
            (
                r.server_id,
                r.name,
                r.hostname,
                r.station,
                r.country_name,
                r.country_code,
                r.city,
                r.load,
                r.port,
                r.status.value,
                r.latency_ms,
                r.tested_at,
                r.error_msg,
            )
            for r in batch
        ]
        with self._get_connection() as conn:
            conn.executemany(
                """
                INSERT OR REPLACE INTO test_cache (
                    server_id, name, hostname, station, country_name, country_code,
                    city, load, port, status, latency_ms, tested_at, error_msg
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
                """,
                records,
            )
            conn.commit()

    def clear_cache(self):
        """Delete all cached test results."""
        with self._get_connection() as conn:
            conn.execute("DELETE FROM test_cache;")
            conn.commit()

    def prune_stale(self, ttl_seconds: float):
        """Remove entries older than ttl_seconds."""
        min_timestamp = time.time() - ttl_seconds
        with self._get_connection() as conn:
            conn.execute("DELETE FROM test_cache WHERE tested_at < ?;", (min_timestamp,))
            conn.commit()

    def count_total(self) -> int:
        """Count total rows in the cache table."""
        with self._get_connection() as conn:
            res = conn.execute("SELECT COUNT(*) FROM test_cache;").fetchone()
            return res[0] if res else 0
