//! Local SQLite intelligence store.

use std::path::Path;
use std::sync::Arc;

use codepulse_protocol::{
    CallerCount, CompareStaticRuntime, FunctionRuntimeSummary, HotPathEntry, SymbolId,
    UncoveredSymbol,
};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS builds (
              id TEXT PRIMARY KEY,
              git_sha TEXT,
              indexed_at_ms INTEGER NOT NULL,
              root_path TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS symbols (
              id TEXT PRIMARY KEY,
              build_id TEXT,
              language TEXT NOT NULL,
              path TEXT NOT NULL,
              qualname TEXT NOT NULL,
              kind TEXT NOT NULL DEFAULT 'function',
              start_line INTEGER NOT NULL DEFAULT 0,
              end_line INTEGER NOT NULL DEFAULT 0,
              param_count INTEGER NOT NULL DEFAULT 0,
              complexity INTEGER NOT NULL DEFAULT 1,
              syntactic_callee_count INTEGER NOT NULL DEFAULT 0,
              UNIQUE(build_id, language, path, qualname)
            );

            CREATE TABLE IF NOT EXISTS runtime_sessions (
              id TEXT PRIMARY KEY,
              started_at_ms INTEGER NOT NULL,
              ended_at_ms INTEGER,
              language TEXT,
              process_id INTEGER,
              command TEXT
            );

            CREATE TABLE IF NOT EXISTS runtime_stats (
              session_id TEXT NOT NULL,
              symbol_id TEXT NOT NULL,
              window_start_ms INTEGER NOT NULL,
              window_end_ms INTEGER NOT NULL,
              invocations INTEGER NOT NULL DEFAULT 0,
              exceptions INTEGER NOT NULL DEFAULT 0,
              duration_ns_p50 INTEGER NOT NULL DEFAULT 0,
              duration_ns_p95 INTEGER NOT NULL DEFAULT 0,
              PRIMARY KEY (session_id, symbol_id, window_start_ms)
            );

            CREATE TABLE IF NOT EXISTS edges (
              session_id TEXT NOT NULL,
              caller_symbol_id TEXT NOT NULL,
              callee_symbol_id TEXT NOT NULL,
              window_start_ms INTEGER NOT NULL,
              count INTEGER NOT NULL DEFAULT 0,
              PRIMARY KEY (session_id, caller_symbol_id, callee_symbol_id, window_start_ms)
            );

            CREATE TABLE IF NOT EXISTS probe_windows (
              id TEXT PRIMARY KEY,
              session_id TEXT,
              started_at_ms INTEGER NOT NULL,
              duration_s INTEGER NOT NULL,
              ended_at_ms INTEGER,
              status TEXT NOT NULL,
              targets_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS static_edges (
              build_id TEXT NOT NULL,
              caller_symbol_id TEXT NOT NULL,
              callee_symbol_id TEXT NOT NULL,
              uncertain INTEGER NOT NULL DEFAULT 0,
              PRIMARY KEY (build_id, caller_symbol_id, callee_symbol_id)
            );

            CREATE INDEX IF NOT EXISTS idx_symbols_lang_path ON symbols(language, path);
            CREATE INDEX IF NOT EXISTS idx_runtime_stats_session ON runtime_stats(session_id);
            CREATE INDEX IF NOT EXISTS idx_edges_callee ON edges(callee_symbol_id);
            "#,
        )?;
        Ok(())
    }

    pub fn symbol_hash(symbol: &SymbolId) -> String {
        let mut hasher = Sha256::new();
        hasher.update(symbol.language.as_bytes());
        hasher.update(b"\0");
        hasher.update(symbol.path.as_bytes());
        hasher.update(b"\0");
        hasher.update(symbol.qualname.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn ensure_session(
        &self,
        session_id: &str,
        language: Option<&str>,
        process_id: u32,
        started_at_ms: u64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT INTO runtime_sessions (id, started_at_ms, language, process_id)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
              language = COALESCE(excluded.language, runtime_sessions.language),
              process_id = excluded.process_id
            "#,
            params![session_id, started_at_ms as i64, language, process_id as i64],
        )?;
        Ok(())
    }

    pub fn ensure_symbol(&self, symbol: &SymbolId) -> Result<String> {
        let id = Self::symbol_hash(symbol);
        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT INTO symbols (id, build_id, language, path, qualname, kind)
            VALUES (?1, NULL, ?2, ?3, ?4, 'function')
            ON CONFLICT(id) DO NOTHING
            "#,
            params![id, symbol.language, symbol.path, symbol.qualname],
        )?;
        Ok(id)
    }

    pub fn upsert_runtime_stat(
        &self,
        session_id: &str,
        symbol_id: &str,
        window_start_ms: u64,
        window_end_ms: u64,
        invocations: u64,
        exceptions: u64,
        duration_ns_p50: u64,
        duration_ns_p95: u64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT INTO runtime_stats (
              session_id, symbol_id, window_start_ms, window_end_ms,
              invocations, exceptions, duration_ns_p50, duration_ns_p95
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(session_id, symbol_id, window_start_ms) DO UPDATE SET
              window_end_ms = excluded.window_end_ms,
              invocations = runtime_stats.invocations + excluded.invocations,
              exceptions = runtime_stats.exceptions + excluded.exceptions,
              duration_ns_p50 = excluded.duration_ns_p50,
              duration_ns_p95 = excluded.duration_ns_p95
            "#,
            params![
                session_id,
                symbol_id,
                window_start_ms as i64,
                window_end_ms as i64,
                invocations as i64,
                exceptions as i64,
                duration_ns_p50 as i64,
                duration_ns_p95 as i64,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_edge(
        &self,
        session_id: &str,
        caller_id: &str,
        callee_id: &str,
        window_start_ms: u64,
        count: u64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT INTO edges (session_id, caller_symbol_id, callee_symbol_id, window_start_ms, count)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(session_id, caller_symbol_id, callee_symbol_id, window_start_ms) DO UPDATE SET
              count = edges.count + excluded.count
            "#,
            params![
                session_id,
                caller_id,
                callee_id,
                window_start_ms as i64,
                count as i64
            ],
        )?;
        Ok(())
    }

    pub fn begin_build(&self, root_path: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO builds (id, git_sha, indexed_at_ms, root_path) VALUES (?1, NULL, ?2, ?3)",
            params![id, now as i64, root_path],
        )?;
        Ok(id)
    }

    pub fn clear_build_symbols(&self, build_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM static_edges WHERE build_id = ?1", params![build_id])?;
        conn.execute("DELETE FROM symbols WHERE build_id = ?1", params![build_id])?;
        Ok(())
    }

    pub fn insert_indexed_symbol(
        &self,
        build_id: &str,
        symbol: &SymbolId,
        kind: &str,
        start_line: u32,
        end_line: u32,
        param_count: u32,
        complexity: u32,
        syntactic_callee_count: u32,
    ) -> Result<String> {
        let id = Self::symbol_hash(symbol);
        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT INTO symbols (
              id, build_id, language, path, qualname, kind,
              start_line, end_line, param_count, complexity, syntactic_callee_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
              build_id = excluded.build_id,
              kind = excluded.kind,
              start_line = excluded.start_line,
              end_line = excluded.end_line,
              param_count = excluded.param_count,
              complexity = excluded.complexity,
              syntactic_callee_count = excluded.syntactic_callee_count
            "#,
            params![
                id,
                build_id,
                symbol.language,
                symbol.path,
                symbol.qualname,
                kind,
                start_line as i64,
                end_line as i64,
                param_count as i64,
                complexity as i64,
                syntactic_callee_count as i64,
            ],
        )?;
        Ok(id)
    }

    pub fn insert_static_edge(
        &self,
        build_id: &str,
        caller_id: &str,
        callee_id: &str,
        uncertain: bool,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT INTO static_edges (build_id, caller_symbol_id, callee_symbol_id, uncertain)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT DO NOTHING
            "#,
            params![build_id, caller_id, callee_id, if uncertain { 1 } else { 0 }],
        )?;
        Ok(())
    }

    pub fn create_probe_window(
        &self,
        id: &str,
        session_id: Option<&str>,
        started_at_ms: u64,
        duration_s: u64,
        targets: &[SymbolId],
    ) -> Result<()> {
        let targets_json = serde_json::to_string(targets)?;
        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT INTO probe_windows (id, session_id, started_at_ms, duration_s, status, targets_json)
            VALUES (?1, ?2, ?3, ?4, 'active', ?5)
            "#,
            params![id, session_id, started_at_ms as i64, duration_s as i64, targets_json],
        )?;
        Ok(())
    }

    pub fn update_probe_status(
        &self,
        id: &str,
        status: &str,
        ended_at_ms: Option<u64>,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE probe_windows SET status = ?1, ended_at_ms = ?2 WHERE id = ?3",
            params![status, ended_at_ms.map(|v| v as i64), id],
        )?;
        Ok(())
    }

    pub fn active_probe_windows(&self, session_id: Option<&str>) -> Result<Vec<ProbeWindowRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, session_id, started_at_ms, duration_s, ended_at_ms, status, targets_json
            FROM probe_windows
            WHERE status = 'active'
              AND (?1 IS NULL OR session_id IS NULL OR session_id = ?1)
            ORDER BY started_at_ms DESC
            "#,
        )?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok(ProbeWindowRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    started_at_ms: row.get::<_, i64>(2)? as u64,
                    duration_s: row.get::<_, i64>(3)? as u64,
                    ended_at_ms: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                    status: row.get(5)?,
                    targets_json: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_probe_window(&self, id: &str) -> Result<Option<ProbeWindowRow>> {
        let conn = self.conn.lock();
        conn.query_row(
            r#"
            SELECT id, session_id, started_at_ms, duration_s, ended_at_ms, status, targets_json
            FROM probe_windows WHERE id = ?1
            "#,
            params![id],
            |row| {
                Ok(ProbeWindowRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    started_at_ms: row.get::<_, i64>(2)? as u64,
                    duration_s: row.get::<_, i64>(3)? as u64,
                    ended_at_ms: row.get::<_, Option<i64>>(4)?.map(|v| v as u64),
                    status: row.get(5)?,
                    targets_json: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn hot_paths(
        &self,
        session_id: Option<&str>,
        limit: usize,
        metric: &str,
    ) -> Result<Vec<HotPathEntry>> {
        let order = match metric {
            "duration_p95" => "SUM(duration_ns_p95)",
            "exceptions" => "SUM(exceptions)",
            _ => "SUM(invocations)",
        };
        let sql = format!(
            r#"
            SELECT s.language, s.path, s.qualname, {order} AS value
            FROM runtime_stats r
            JOIN symbols s ON s.id = r.symbol_id
            WHERE (?1 IS NULL OR r.session_id = ?1)
            GROUP BY s.id
            ORDER BY value DESC
            LIMIT ?2
            "#
        );
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![session_id, limit as i64], |row| {
                Ok(HotPathEntry {
                    symbol: SymbolId {
                        language: row.get(0)?,
                        path: row.get(1)?,
                        qualname: row.get(2)?,
                    },
                    value: row.get::<_, i64>(3)? as f64,
                    metric: metric.to_string(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn function_runtime_summary(
        &self,
        symbol: &SymbolId,
        session_id: Option<&str>,
    ) -> Result<Option<FunctionRuntimeSummary>> {
        let symbol_id = Self::symbol_hash(symbol);
        let conn = self.conn.lock();
        let stats = conn
            .query_row(
                r#"
                SELECT
                  COALESCE(SUM(invocations), 0),
                  COALESCE(SUM(exceptions), 0),
                  COALESCE(AVG(duration_ns_p50), 0),
                  COALESCE(MAX(duration_ns_p95), 0)
                FROM runtime_stats
                WHERE symbol_id = ?1 AND (?2 IS NULL OR session_id = ?2)
                "#,
                params![symbol_id, session_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? as u64,
                        row.get::<_, i64>(1)? as u64,
                        row.get::<_, f64>(2)?,
                        row.get::<_, i64>(3)? as u64,
                    ))
                },
            )
            .optional()?;

        let Some((invocations, exceptions, p50, p95)) = stats else {
            return Ok(None);
        };
        if invocations == 0 && exceptions == 0 {
            return Ok(None);
        }

        let mut stmt = conn.prepare(
            r#"
            SELECT s.language, s.path, s.qualname, SUM(e.count) AS cnt
            FROM edges e
            JOIN symbols s ON s.id = e.caller_symbol_id
            WHERE e.callee_symbol_id = ?1 AND (?2 IS NULL OR e.session_id = ?2)
            GROUP BY s.id
            ORDER BY cnt DESC
            LIMIT 10
            "#,
        )?;
        let callers: Vec<CallerCount> = stmt
            .query_map(params![symbol_id, session_id], |row| {
                let sym = SymbolId {
                    language: row.get(0)?,
                    path: row.get(1)?,
                    qualname: row.get(2)?,
                };
                Ok(CallerCount {
                    qualname: sym.qualname.clone(),
                    count: row.get::<_, i64>(3)? as u64,
                    symbol: Some(sym),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(Some(FunctionRuntimeSummary {
            symbol: symbol.clone(),
            invocations,
            exceptions,
            duration_ms_p50: p50 / 1_000_000.0,
            duration_ms_p95: p95 as f64 / 1_000_000.0,
            distinct_callers: callers.len() as u64,
            top_callers: callers,
        }))
    }

    pub fn actual_callers(
        &self,
        symbol: &SymbolId,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<CallerCount>> {
        let symbol_id = Self::symbol_hash(symbol);
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"
            SELECT s.language, s.path, s.qualname, SUM(e.count) AS cnt
            FROM edges e
            JOIN symbols s ON s.id = e.caller_symbol_id
            WHERE e.callee_symbol_id = ?1 AND (?2 IS NULL OR e.session_id = ?2)
            GROUP BY s.id
            ORDER BY cnt DESC
            LIMIT ?3
            "#,
        )?;
        let rows = stmt
            .query_map(params![symbol_id, session_id, limit as i64], |row| {
                let sym = SymbolId {
                    language: row.get(0)?,
                    path: row.get(1)?,
                    qualname: row.get(2)?,
                };
                Ok(CallerCount {
                    qualname: sym.qualname.clone(),
                    count: row.get::<_, i64>(3)? as u64,
                    symbol: Some(sym),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn actual_callees(
        &self,
        symbol: &SymbolId,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<CallerCount>> {
        let symbol_id = Self::symbol_hash(symbol);
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"
            SELECT s.language, s.path, s.qualname, SUM(e.count) AS cnt
            FROM edges e
            JOIN symbols s ON s.id = e.callee_symbol_id
            WHERE e.caller_symbol_id = ?1 AND (?2 IS NULL OR e.session_id = ?2)
            GROUP BY s.id
            ORDER BY cnt DESC
            LIMIT ?3
            "#,
        )?;
        let rows = stmt
            .query_map(params![symbol_id, session_id, limit as i64], |row| {
                let sym = SymbolId {
                    language: row.get(0)?,
                    path: row.get(1)?,
                    qualname: row.get(2)?,
                };
                Ok(CallerCount {
                    qualname: sym.qualname.clone(),
                    count: row.get::<_, i64>(3)? as u64,
                    symbol: Some(sym),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn static_summary(&self, symbol: &SymbolId) -> Result<Option<codepulse_protocol::StaticSummary>> {
        let id = Self::symbol_hash(symbol);
        let conn = self.conn.lock();
        conn.query_row(
            r#"
            SELECT language, path, qualname, kind, complexity, param_count,
                   (end_line - start_line + 1), syntactic_callee_count, start_line, end_line
            FROM symbols WHERE id = ?1
            "#,
            params![id],
            |row| {
                Ok(codepulse_protocol::StaticSummary {
                    symbol: SymbolId {
                        language: row.get(0)?,
                        path: row.get(1)?,
                        qualname: row.get(2)?,
                    },
                    kind: row.get(3)?,
                    complexity: row.get(4)?,
                    param_count: row.get(5)?,
                    lines: row.get(6)?,
                    syntactic_callee_count: row.get(7)?,
                    start_line: row.get(8)?,
                    end_line: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn compare_static_vs_runtime(
        &self,
        symbol: &SymbolId,
        session_id: Option<&str>,
    ) -> Result<CompareStaticRuntime> {
        let symbol_id = Self::symbol_hash(symbol);
        let conn = self.conn.lock();

        let mut static_callees = Vec::new();
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT s.qualname FROM static_edges e
                JOIN symbols s ON s.id = e.callee_symbol_id
                WHERE e.caller_symbol_id = ?1
                "#,
            )?;
            let rows = stmt.query_map(params![symbol_id], |row| row.get::<_, String>(0))?;
            for row in rows {
                static_callees.push(row?);
            }
        }

        let mut observed = Vec::new();
        {
            let mut stmt = conn.prepare(
                r#"
                SELECT s.qualname FROM edges e
                JOIN symbols s ON s.id = e.callee_symbol_id
                WHERE e.caller_symbol_id = ?1 AND (?2 IS NULL OR e.session_id = ?2)
                GROUP BY s.id
                "#,
            )?;
            let rows =
                stmt.query_map(params![symbol_id, session_id], |row| row.get::<_, String>(0))?;
            for row in rows {
                observed.push(row?);
            }
        }

        let never: Vec<String> = static_callees
            .iter()
            .filter(|c| !observed.contains(c))
            .cloned()
            .collect();
        let runtime_only: Vec<String> = observed
            .iter()
            .filter(|c| !static_callees.contains(c))
            .cloned()
            .collect();

        Ok(CompareStaticRuntime {
            static_callees: static_callees.len() as u64,
            observed_callees: observed.len() as u64,
            never_observed_static_callees: never,
            runtime_only_callees: runtime_only,
        })
    }

    pub fn uncovered_hot_symbols(
        &self,
        min_complexity: i64,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<UncoveredSymbol>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"
            SELECT s.language, s.path, s.qualname, s.complexity
            FROM symbols s
            WHERE s.build_id IS NOT NULL
              AND s.complexity >= ?1
              AND NOT EXISTS (
                SELECT 1 FROM runtime_stats r
                WHERE r.symbol_id = s.id
                  AND r.invocations > 0
                  AND (?2 IS NULL OR r.session_id = ?2)
              )
            ORDER BY s.complexity DESC
            LIMIT ?3
            "#,
        )?;
        let rows = stmt
            .query_map(params![min_complexity, session_id, limit as i64], |row| {
                let path: String = row.get(1)?;
                Ok(UncoveredSymbol {
                    symbol: SymbolId {
                        language: row.get(0)?,
                        path: path.clone(),
                        qualname: row.get(2)?,
                    },
                    complexity: row.get(3)?,
                    path,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[derive(Debug, Clone)]
pub struct ProbeWindowRow {
    pub id: String,
    pub session_id: Option<String>,
    pub started_at_ms: u64,
    pub duration_s: u64,
    pub ended_at_ms: Option<u64>,
    pub status: String,
    pub targets_json: String,
}

pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepulse_protocol::SymbolId;

    #[test]
    fn ingest_and_hot_paths() {
        let store = Store::open_in_memory().unwrap();
        let sym = SymbolId::new("python", "app.py", "foo");
        let sid = store.ensure_symbol(&sym).unwrap();
        store.ensure_session("sess1", Some("python"), 1, 1000).unwrap();
        store
            .upsert_runtime_stat("sess1", &sid, 1000, 2000, 42, 1, 10_000, 20_000)
            .unwrap();
        let hot = store.hot_paths(Some("sess1"), 10, "invocations").unwrap();
        assert_eq!(hot.len(), 1);
        assert_eq!(hot[0].value, 42.0);
        assert_eq!(hot[0].symbol.qualname, "foo");
    }

    #[test]
    fn callers() {
        let store = Store::open_in_memory().unwrap();
        let callee = SymbolId::new("python", "a.py", "target");
        let caller = SymbolId::new("python", "b.py", "caller");
        let callee_id = store.ensure_symbol(&callee).unwrap();
        let caller_id = store.ensure_symbol(&caller).unwrap();
        store.ensure_session("s", Some("python"), 1, 0).unwrap();
        store.upsert_edge("s", &caller_id, &callee_id, 0, 9).unwrap();
        let callers = store.actual_callers(&callee, Some("s"), 10).unwrap();
        assert_eq!(callers.len(), 1);
        assert_eq!(callers[0].count, 9);
    }
}
