//! Collections repository.

use super::db::{now_ms, Database};
use crate::error::{AppError, AppResult};
use crate::models::{Collection, HttpRequest, SavedRequest};
use rusqlite::params;

pub fn create(db: &Database, name: &str) -> AppResult<Collection> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidRequest(
            "the collection name is empty".into(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    db.with(|conn| {
        conn.execute(
            "INSERT INTO collections (id, name, created_at) VALUES (?1, ?2, ?3)",
            params![id, name, now_ms()],
        )?;
        Ok(())
    })?;
    Ok(Collection {
        id,
        name: name.to_string(),
        requests: vec![],
    })
}

pub fn rename(db: &Database, id: &str, name: &str) -> AppResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidRequest(
            "the collection name is empty".into(),
        ));
    }
    db.with(|conn| {
        let changed = conn.execute(
            "UPDATE collections SET name = ?2 WHERE id = ?1",
            params![id, name],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound(format!("collection {id}")));
        }
        Ok(())
    })
}

pub fn delete(db: &Database, id: &str) -> AppResult<()> {
    db.with(|conn| {
        let changed = conn.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(AppError::NotFound(format!("collection {id}")));
        }
        Ok(())
    })
}

pub fn list(db: &Database) -> AppResult<Vec<Collection>> {
    db.with(|conn| {
        let mut stmt =
            conn.prepare("SELECT id, name FROM collections ORDER BY name COLLATE NOCASE")?;
        let collections = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut stmt = conn.prepare(
            "SELECT id, name, request_json FROM saved_requests
             WHERE collection_id = ?1 ORDER BY created_at",
        )?;

        let mut out = Vec::with_capacity(collections.len());
        for (id, name) in collections {
            let rows = stmt.query_map(params![id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            let mut requests = Vec::new();
            for row in rows {
                let (request_id, request_name, request_json) = row?;
                requests.push(SavedRequest {
                    id: request_id,
                    collection_id: id.clone(),
                    name: request_name,
                    request: serde_json::from_str(&request_json)?,
                });
            }
            out.push(Collection { id, name, requests });
        }
        Ok(out)
    })
}

/// Saves a request into a collection.
///
/// The identity of a saved request is its own row id, never `request.id` (a tab
/// keeps the same `request.id` for its whole life, so using it as the key made
/// every later save overwrite the first one). `saved_request_id` is `Some` only
/// when the caller explicitly wants to update that existing entry; `None`
/// always creates a new one.
pub fn save_request(
    db: &Database,
    collection_id: &str,
    name: &str,
    request: &HttpRequest,
    saved_request_id: Option<&str>,
) -> AppResult<SavedRequest> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidRequest("the request name is empty".into()));
    }
    let request_json = serde_json::to_string(request)?;

    let id = db.with(|conn| {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM collections WHERE id = ?1",
            params![collection_id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(AppError::NotFound(format!("collection {collection_id}")));
        }

        match saved_request_id {
            Some(existing) => {
                let changed = conn.execute(
                    "UPDATE saved_requests
                     SET collection_id = ?2, name = ?3, request_json = ?4
                     WHERE id = ?1",
                    params![existing, collection_id, name, request_json],
                )?;
                if changed == 0 {
                    return Err(AppError::NotFound(format!("request {existing}")));
                }
                Ok(existing.to_string())
            }
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO saved_requests (id, collection_id, name, request_json, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, collection_id, name, request_json, now_ms()],
                )?;
                Ok(id)
            }
        }
    })?;

    Ok(SavedRequest {
        id,
        collection_id: collection_id.to_string(),
        name: name.to_string(),
        request: request.clone(),
    })
}

pub fn delete_request(db: &Database, id: &str) -> AppResult<()> {
    db.with(|conn| {
        let changed = conn.execute("DELETE FROM saved_requests WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(AppError::NotFound(format!("request {id}")));
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Auth, HttpMethod, RequestBody};

    fn request(id: &str, url: &str) -> HttpRequest {
        HttpRequest {
            id: id.into(),
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
    fn creates_and_lists_collections() {
        let db = Database::open_in_memory().expect("db");
        create(&db, "Beta").expect("create");
        create(&db, "Alpha").expect("create");
        let names: Vec<_> = list(&db)
            .expect("list")
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, vec!["Alpha", "Beta"]);
    }

    #[test]
    fn rejects_a_blank_name() {
        let db = Database::open_in_memory().expect("db");
        assert_eq!(
            create(&db, "   ").expect_err("should reject").kind(),
            "invalidRequest"
        );
    }

    #[test]
    fn saves_a_request_into_a_collection() {
        let db = Database::open_in_memory().expect("db");
        let collection = create(&db, "API").expect("create");
        save_request(
            &db,
            &collection.id,
            "List users",
            &request("r1", "https://a.test"),
            None,
        )
        .expect("save");

        let listed = list(&db).expect("list");
        assert_eq!(listed[0].requests.len(), 1);
        assert_eq!(listed[0].requests[0].name, "List users");
        assert_eq!(listed[0].requests[0].request.url, "https://a.test");
    }

    /// Regression: one tab keeps the same `request.id`, so saving it twice used
    /// to overwrite the first entry instead of adding a second one.
    #[test]
    fn saving_the_same_request_id_twice_adds_two_entries() {
        let db = Database::open_in_memory().expect("db");
        let collection = create(&db, "library").expect("create");
        let first = save_request(
            &db,
            &collection.id,
            "get",
            &request("tab-1", "https://a.test"),
            None,
        )
        .expect("save");
        let second = save_request(
            &db,
            &collection.id,
            "post",
            &request("tab-1", "https://b.test"),
            None,
        )
        .expect("save");

        assert_ne!(first.id, second.id);
        let listed = list(&db).expect("list");
        let names: Vec<_> = listed[0].requests.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["get", "post"]);
    }

    #[test]
    fn saving_into_another_collection_keeps_the_original() {
        let db = Database::open_in_memory().expect("db");
        let library = create(&db, "library").expect("create");
        let other = create(&db, "other").expect("create");
        save_request(
            &db,
            &library.id,
            "get",
            &request("tab-1", "https://a.test"),
            None,
        )
        .expect("save");
        save_request(
            &db,
            &other.id,
            "get",
            &request("tab-1", "https://a.test"),
            None,
        )
        .expect("save");

        let listed = list(&db).expect("list");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].requests.len(), 1);
        assert_eq!(listed[1].requests.len(), 1);
    }

    #[test]
    fn updating_a_missing_saved_request_reports_not_found() {
        let db = Database::open_in_memory().expect("db");
        let collection = create(&db, "API").expect("create");
        let err = save_request(
            &db,
            &collection.id,
            "x",
            &request("tab-1", "https://a.test"),
            Some("nope"),
        )
        .expect_err("should fail");
        assert_eq!(err.kind(), "notFound");
    }

    #[test]
    fn saving_with_an_explicit_id_updates_that_entry() {
        let db = Database::open_in_memory().expect("db");
        let collection = create(&db, "API").expect("create");
        let saved = save_request(
            &db,
            &collection.id,
            "v1",
            &request("r1", "https://a.test"),
            None,
        )
        .expect("save");
        save_request(
            &db,
            &collection.id,
            "v2",
            &request("r1", "https://b.test"),
            Some(&saved.id),
        )
        .expect("save");

        let listed = list(&db).expect("list");
        assert_eq!(listed[0].requests.len(), 1);
        assert_eq!(listed[0].requests[0].name, "v2");
        assert_eq!(listed[0].requests[0].request.url, "https://b.test");
    }

    #[test]
    fn saving_into_a_missing_collection_fails() {
        let db = Database::open_in_memory().expect("db");
        let err = save_request(&db, "nope", "x", &request("r1", "https://a.test"), None)
            .expect_err("should fail");
        assert_eq!(err.kind(), "notFound");
    }

    #[test]
    fn deleting_a_collection_removes_its_requests() {
        let db = Database::open_in_memory().expect("db");
        let collection = create(&db, "API").expect("create");
        save_request(
            &db,
            &collection.id,
            "x",
            &request("r1", "https://a.test"),
            None,
        )
        .expect("save");
        delete(&db, &collection.id).expect("delete");

        assert!(list(&db).expect("list").is_empty());
        let count: i64 = db
            .with(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM saved_requests", [], |r| r.get(0))?)
            })
            .expect("count");
        assert_eq!(count, 0);
    }

    #[test]
    fn renames_a_collection() {
        let db = Database::open_in_memory().expect("db");
        let collection = create(&db, "Old").expect("create");
        rename(&db, &collection.id, "New").expect("rename");
        assert_eq!(list(&db).expect("list")[0].name, "New");
    }

    #[test]
    fn deleting_a_missing_request_reports_not_found() {
        let db = Database::open_in_memory().expect("db");
        assert_eq!(
            delete_request(&db, "nope").expect_err("should fail").kind(),
            "notFound"
        );
    }
}
