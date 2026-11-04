//! History repository.

use super::db::{now_ms, Database};
use crate::error::AppResult;
use crate::models::{HistoryEntry, HttpRequest};
use rusqlite::params;

/// Entries kept beyond this count are pruned after each insert.
const MAX_ENTRIES: usize = 500;

pub fn record(
    db: &Database,
    request: &HttpRequest,
    status: Option<u16>,
    duration_ms: Option<u64>,
) -> AppResult<HistoryEntry> {
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        sent_at: now_ms(),
        method: request.method,
        url: request.url.clone(),
        status,
        duration_ms,
        request: request.clone(),
    };
    let request_json = serde_json::to_string(&entry.request)?;

    db.with(|conn| {
        conn.execute(
            "INSERT INTO history (id, sent_at, method, url, status, duration_ms, request_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                entry.sent_at,
                entry.method.as_str(),
                entry.url,
                entry.status,
                entry.duration_ms,
                request_json,
            ],
        )?;
        conn.execute(
            "DELETE FROM history WHERE id NOT IN
                (SELECT id FROM history ORDER BY sent_at DESC LIMIT ?1)",
            params![MAX_ENTRIES as i64],
        )?;
        Ok(())
    })?;

    Ok(entry)
}

pub fn list(db: &Database, limit: u32) -> AppResult<Vec<HistoryEntry>> {
    db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, sent_at, method, url, status, duration_ms, request_json
             FROM history ORDER BY sent_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<u16>>(4)?,
                row.get::<_, Option<u64>>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let (id, sent_at, status, duration_ms, request_json) = row?;
            let request: HttpRequest = serde_json::from_str(&request_json)?;
            entries.push(HistoryEntry {
                id,
                sent_at,
                method: request.method,
                url: request.url.clone(),
                status,
                duration_ms,
                request,
            });
        }
        Ok(entries)
    })
}

pub fn clear(db: &Database) -> AppResult<()> {
    db.with(|conn| {
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Auth, HttpMethod, RequestBody};

    fn request(url: &str) -> HttpRequest {
        HttpRequest {
            id: "req".into(),
            method: HttpMethod::Get,
            url: url.into(),
            params: vec![],
            headers: vec![],
            body: RequestBody::None,
            auth: Auth::None,
            timeout_ms: None,
            name: None,
        }
    }

    #[test]
    fn records_and_lists_newest_first() {
        let db = Database::open_in_memory().expect("db");
        record(&db, &request("https://a.test"), Some(200), Some(12)).expect("record a");
        std::thread::sleep(std::time::Duration::from_millis(2));
        record(&db, &request("https://b.test"), Some(404), Some(34)).expect("record b");

        let entries = list(&db, 10).expect("list");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].url, "https://b.test");
        assert_eq!(entries[0].status, Some(404));
        assert_eq!(entries[1].url, "https://a.test");
    }

    #[test]
    fn respects_the_limit() {
        let db = Database::open_in_memory().expect("db");
        for i in 0..5 {
            record(&db, &request(&format!("https://{i}.test")), Some(200), None).expect("record");
        }
        assert_eq!(list(&db, 3).expect("list").len(), 3);
    }

    #[test]
    fn clear_removes_everything() {
        let db = Database::open_in_memory().expect("db");
        record(&db, &request("https://a.test"), Some(200), None).expect("record");
        clear(&db).expect("clear");
        assert!(list(&db, 10).expect("list").is_empty());
    }

    #[test]
    fn a_failed_request_is_recorded_without_a_status() {
        let db = Database::open_in_memory().expect("db");
        record(&db, &request("https://a.test"), None, None).expect("record");
        let entries = list(&db, 10).expect("list");
        assert_eq!(entries[0].status, None);
    }
}
