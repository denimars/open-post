//! SQLite connection handling and schema migrations.

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// How long a write waits for another connection to release its lock before
/// giving up. A crashed instance can leave the file locked for a moment.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Checkpoint the WAL once it grows past this many pages (~1 MB at 4 KB pages),
/// so an unclean exit cannot leave a large `-wal` file behind.
const WAL_AUTOCHECKPOINT_PAGES: i64 = 256;

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

    /// Fallback used when the database file cannot be opened at all: the app
    /// still starts, but nothing is persisted.
    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        conn.busy_timeout(BUSY_TIMEOUT)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "wal_autocheckpoint", WAL_AUTOCHECKPOINT_PAGES)?;
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

    /// Folds the WAL back into the main database file and truncates it.
    /// Called when the app exits so a later crash cannot inherit a huge `-wal`.
    pub fn checkpoint(&self) {
        let _ = self.with(|conn| {
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
            Ok(())
        });
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

/// Result of opening the database at startup.
pub struct Opened {
    pub db: Database,
    /// Set when the stored data could not be used as it was; shown in the UI.
    pub warning: Option<String>,
}

/// Opens the database, recovering instead of failing.
///
/// A crash can leave the file (or its `-wal`) in a state SQLite refuses to
/// open. Failing here used to abort startup before the window was shown, so
/// the app looked like it did nothing at all when launched. Now the broken
/// file is moved aside, and as a last resort the app runs on an in-memory
/// database so the window always appears.
pub fn open_or_recover(path: &Path) -> AppResult<Opened> {
    let first = match Database::open(path) {
        Ok(db) => {
            return Ok(Opened { db, warning: None });
        }
        Err(e) => e.to_string(),
    };

    let moved = quarantine(path);

    match Database::open(path) {
        Ok(db) => {
            let detail = match moved {
                Ok(Some(to)) => format!("The previous file was kept as {}.", to.display()),
                Ok(None) => String::new(),
                Err(e) => format!("The previous file could not be moved aside: {e}."),
            };
            Ok(Opened {
                db,
                warning: Some(format!(
                    "Collections, history and environments were reset: the database \
                     could not be opened ({first}). {detail}"
                )),
            })
        }
        Err(second) => Ok(Opened {
            db: Database::open_in_memory()?,
            warning: Some(format!(
                "Running without storage: the database could not be opened ({first}), \
                 and creating a new one failed too ({second}). Collections, history and \
                 environments will not be saved."
            )),
        }),
    }
}

/// Moves `path` and its WAL sidecars out of the way. Returns the new path of
/// the main file, or `None` when there was nothing to move.
fn quarantine(path: &Path) -> std::io::Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let stamp = now_ms();
    let name = path.file_name().map(|n| n.to_owned()).unwrap_or_default();
    let target = path.with_file_name(format!("{}.corrupt-{stamp}", name.to_string_lossy()));
    std::fs::rename(path, &target)?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = path.with_file_name(format!("{}{suffix}", name.to_string_lossy()));
        if sidecar.exists() {
            let _ = std::fs::rename(
                &sidecar,
                target.with_file_name(format!(
                    "{}{suffix}",
                    target.file_name().unwrap_or_default().to_string_lossy()
                )),
            );
        }
    }
    Ok(Some(target))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("open-post-{name}-{}", now_ms()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn opens_a_fresh_database_without_warning() {
        let dir = temp_dir("fresh");
        let opened = open_or_recover(&dir.join("open-post.db")).expect("opened");
        assert!(opened.warning.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn recovers_from_an_unreadable_file_and_still_starts() {
        let dir = temp_dir("corrupt");
        let path = dir.join("open-post.db");
        std::fs::write(&path, b"this is not a sqlite database").expect("write");

        let opened = open_or_recover(&path).expect("recovered");
        let warning = opened.warning.expect("a warning");
        assert!(warning.contains("reset"), "unexpected warning: {warning}");

        // The replacement database is usable.
        opened
            .db
            .with(|conn| {
                conn.execute_batch("INSERT INTO app_state (key, value) VALUES ('k', 'v');")?;
                Ok(())
            })
            .expect("usable");

        // The old file was kept, not deleted.
        let kept = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().contains(".corrupt-"));
        assert!(kept, "the broken file should have been kept aside");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn checkpoint_truncates_the_wal() {
        let dir = temp_dir("wal");
        let path = dir.join("open-post.db");
        let opened = open_or_recover(&path).expect("opened");
        opened
            .db
            .with(|conn| {
                for i in 0..200 {
                    conn.execute(
                        "INSERT INTO app_state (key, value) VALUES (?1, ?2)",
                        rusqlite::params![format!("k{i}"), "v".repeat(2048)],
                    )?;
                }
                Ok(())
            })
            .expect("writes");

        opened.db.checkpoint();

        let wal = path.with_file_name("open-post.db-wal");
        let size = std::fs::metadata(&wal).map(|m| m.len()).unwrap_or(0);
        assert_eq!(size, 0, "the WAL should be empty after a checkpoint");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
