use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{TestResult, TestStatus};

const DEFAULT_CACHE_DIR: &str = ".cache";
const DEFAULT_DB_FILE: &str = "nord_test_cache.db";

#[derive(Clone)]
pub struct CacheManager {
    conn: Arc<Mutex<Connection>>,
    #[allow(dead_code)]
    db_path: PathBuf,
}

impl CacheManager {
    pub fn new(custom_path: Option<&Path>) -> Result<Self> {
        let db_path = match custom_path {
            Some(p) => p.to_path_buf(),
            None => {
                let dir = PathBuf::from(DEFAULT_CACHE_DIR);
                let _ = fs::create_dir_all(&dir);
                dir.join(DEFAULT_DB_FILE)
            }
        };

        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", db_path))?;

        // Enable WAL mode for high concurrent throughput
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        let manager = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path,
        };
        manager.init_db()?;
        Ok(manager)
    }

    fn init_db(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS test_cache (
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
            CREATE INDEX IF NOT EXISTS idx_tested_at ON test_cache(tested_at);
            CREATE INDEX IF NOT EXISTS idx_status ON test_cache(status);
            CREATE INDEX IF NOT EXISTS idx_country ON test_cache(country_code);",
        )?;
        Ok(())
    }

    pub fn get_fresh_cache(&self, ttl_seconds: f64) -> Result<HashMap<i64, TestResult>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let min_timestamp = now - ttl_seconds;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT server_id, name, hostname, station, country_name, country_code,
                    city, load, port, status, latency_ms, tested_at, error_msg
             FROM test_cache
             WHERE tested_at >= ?1",
        )?;

        let mut results = HashMap::new();
        let rows = stmt.query_map(params![min_timestamp], |row| {
            let status_str: String = row.get(9)?;
            let status = status_str.parse::<TestStatus>().unwrap_or(TestStatus::Error);

            Ok(TestResult {
                server_id: row.get(0)?,
                name: row.get(1)?,
                hostname: row.get(2)?,
                station: row.get(3)?,
                country_name: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                country_code: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                city: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                load: row.get::<_, Option<i32>>(7)?.unwrap_or(0),
                port: row.get(8)?,
                status,
                latency_ms: row.get(10)?,
                tested_at: row.get(11)?,
                is_cached: true,
                features: Vec::new(),
                error_msg: row.get(12)?,
            })
        })?;

        for r in rows {
            if let Ok(res) = r {
                results.insert(res.server_id, res);
            }
        }

        Ok(results)
    }

    pub fn save_batch(&self, batch: &[TestResult]) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO test_cache (
                    server_id, name, hostname, station, country_name, country_code,
                    city, load, port, status, latency_ms, tested_at, error_msg
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;

            for r in batch {
                stmt.execute(params![
                    r.server_id,
                    r.name,
                    r.hostname,
                    r.station,
                    r.country_name,
                    r.country_code,
                    r.city,
                    r.load,
                    r.port,
                    r.status.as_str(),
                    r.latency_ms,
                    r.tested_at,
                    r.error_msg,
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM test_cache;", [])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn prune_stale(&self, ttl_seconds: f64) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let min_timestamp = now - ttl_seconds;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM test_cache WHERE tested_at < ?1;",
            params![min_timestamp],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn count_total(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM test_cache;", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}
