//! SQLite connection handling and schema migrations.

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Serialized access to the single SQLite connection.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Storage(format!("could not create the data directory: {e}"))
            })?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Runs `f` with the connection locked.
    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Storage("the database lock was poisoned".into()))?;
        f(&guard)
    }

    fn migrate(&self) -> AppResult<()> {
        self.with(|conn| {
            conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS collections (
                    id         TEXT PRIMARY KEY,
                    name       TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS saved_requests (
                    id            TEXT PRIMARY KEY,
                    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
                    name          TEXT NOT NULL,
                    request_json  TEXT NOT NULL,
                    created_at    INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_saved_requests_collection
                    ON saved_requests(collection_id);

                CREATE TABLE IF NOT EXISTS history (
                    id           TEXT PRIMARY KEY,
                    sent_at      INTEGER NOT NULL,
                    method       TEXT NOT NULL,
                    url          TEXT NOT NULL,
                    status       INTEGER,
                    duration_ms  INTEGER,
                    request_json TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_history_sent_at ON history(sent_at DESC);

                CREATE TABLE IF NOT EXISTS environments (
                    id             TEXT PRIMARY KEY,
                    name           TEXT NOT NULL,
                    variables_json TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS app_state (
                    key   TEXT PRIMARY KEY,
                    value TEXT
                );
                "#,
            )?;
            Ok(())
        })
    }
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
