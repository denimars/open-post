//! Environments repository, plus the active-environment setting.

use super::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{Environment, KeyValue};
use rusqlite::params;

const ACTIVE_KEY: &str = "active_environment_id";

pub fn list(db: &Database) -> AppResult<Vec<Environment>> {
    db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, variables_json FROM environments ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, name, variables_json) = row?;
            out.push(Environment {
                id,
                name,
                variables: serde_json::from_str(&variables_json)?,
            });
        }
        Ok(out)
    })
}

/// Inserts or replaces the environment.
pub fn save(db: &Database, env: &Environment) -> AppResult<Environment> {
    let name = env.name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidRequest(
            "the environment name is empty".into(),
        ));
    }
    let variables_json = serde_json::to_string(&env.variables)?;
    let id = if env.id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        env.id.clone()
    };

    db.with(|conn| {
        conn.execute(
            "INSERT INTO environments (id, name, variables_json) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
                 name           = excluded.name,
                 variables_json = excluded.variables_json",
            params![id, name, variables_json],
        )?;
        Ok(())
    })?;

    Ok(Environment {
        id,
        name: name.to_string(),
        variables: env.variables.clone(),
    })
}

pub fn delete(db: &Database, id: &str) -> AppResult<()> {
    db.with(|conn| {
        let changed = conn.execute("DELETE FROM environments WHERE id = ?1", params![id])?;
        if changed == 0 {
            return Err(AppError::NotFound(format!("environment {id}")));
        }
        conn.execute(
            "DELETE FROM app_state WHERE key = ?1 AND value = ?2",
            params![ACTIVE_KEY, id],
        )?;
        Ok(())
    })
}

pub fn get(db: &Database, id: &str) -> AppResult<Environment> {
    db.with(|conn| {
        let (name, variables_json) = conn
            .query_row(
                "SELECT name, variables_json FROM environments WHERE id = ?1",
                params![id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("environment {id}"))
                }
                other => AppError::from(other),
            })?;
        Ok(Environment {
            id: id.to_string(),
            name,
            variables: serde_json::from_str(&variables_json)?,
        })
    })
}

/// `None` clears the selection.
pub fn set_active(db: &Database, id: Option<&str>) -> AppResult<()> {
    db.with(|conn| {
        match id {
            Some(id) => {
                conn.execute(
                    "INSERT INTO app_state (key, value) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![ACTIVE_KEY, id],
                )?;
            }
            None => {
                conn.execute("DELETE FROM app_state WHERE key = ?1", params![ACTIVE_KEY])?;
            }
        }
        Ok(())
    })
}

pub fn active_id(db: &Database) -> AppResult<Option<String>> {
    db.with(|conn| {
        let value = conn
            .query_row(
                "SELECT value FROM app_state WHERE key = ?1",
                params![ACTIVE_KEY],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();
        Ok(value)
    })
}

/// Stores an imported environment under a fresh id, making the name unique
/// (`Staging`, `Staging (2)`, …) so an import never overwrites an environment
/// the user already has.
pub fn import(db: &Database, name: &str, variables: &[KeyValue]) -> AppResult<Environment> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidRequest(
            "the environment name is empty".into(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let variables_json = serde_json::to_string(variables)?;

    let unique = db.with(|conn| {
        let unique = unique_environment_name(conn, name)?;
        conn.execute(
            "INSERT INTO environments (id, name, variables_json) VALUES (?1, ?2, ?3)",
            params![id, unique, variables_json],
        )?;
        Ok(unique)
    })?;

    Ok(Environment {
        id,
        name: unique,
        variables: variables.to_vec(),
    })
}

fn unique_environment_name(conn: &rusqlite::Connection, name: &str) -> AppResult<String> {
    let taken = |candidate: &str| -> AppResult<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM environments WHERE name = ?1 COLLATE NOCASE",
            params![candidate],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    };

    if !taken(name)? {
        return Ok(name.to_string());
    }
    for suffix in 2..1000 {
        let candidate = format!("{name} ({suffix})");
        if !taken(&candidate)? {
            return Ok(candidate);
        }
    }
    Err(AppError::Storage(format!(
        "too many environments already named `{name}`"
    )))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KeyValue;

    fn env(id: &str, name: &str) -> Environment {
        Environment {
            id: id.into(),
            name: name.into(),
            variables: vec![KeyValue {
                key: "base_url".into(),
                value: "https://api.test".into(),
                enabled: true,
            }],
        }
    }

    #[test]
    fn saves_and_lists_environments() {
        let db = Database::open_in_memory().expect("db");
        save(&db, &env("e1", "Staging")).expect("save");
        let listed = list(&db).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].variables[0].key, "base_url");
    }

    #[test]
    fn generates_an_id_when_none_is_given() {
        let db = Database::open_in_memory().expect("db");
        let saved = save(&db, &env("", "Local")).expect("save");
        assert!(!saved.id.is_empty());
    }

    #[test]
    fn saving_an_existing_id_updates_it() {
        let db = Database::open_in_memory().expect("db");
        save(&db, &env("e1", "Staging")).expect("save");
        save(&db, &env("e1", "Production")).expect("save");
        let listed = list(&db).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Production");
    }

    #[test]
    fn active_environment_round_trips() {
        let db = Database::open_in_memory().expect("db");
        save(&db, &env("e1", "Staging")).expect("save");
        assert_eq!(active_id(&db).expect("active"), None);
        set_active(&db, Some("e1")).expect("set");
        assert_eq!(active_id(&db).expect("active"), Some("e1".into()));
        set_active(&db, None).expect("clear");
        assert_eq!(active_id(&db).expect("active"), None);
    }

    #[test]
    fn imports_an_environment_under_a_unique_name() {
        let db = Database::open_in_memory().expect("db");
        let variables = vec![KeyValue {
            key: "base_url".into(),
            value: "https://api.test".into(),
            enabled: true,
        }];

        let first = import(&db, "Staging", &variables).expect("import");
        let second = import(&db, "Staging", &variables).expect("import");

        assert_eq!(first.name, "Staging");
        assert_eq!(second.name, "Staging (2)");
        assert_ne!(first.id, second.id);
        assert_eq!(list(&db).expect("list").len(), 2);
        assert_eq!(first.variables[0].key, "base_url");
    }

    /// An import must never overwrite an environment the user already has.
    #[test]
    fn importing_does_not_touch_an_existing_environment() {
        let db = Database::open_in_memory().expect("db");
        save(&db, &env("e1", "Staging")).expect("save");
        import(&db, "Staging", &[]).expect("import");

        let kept = get(&db, "e1").expect("get");
        assert_eq!(kept.name, "Staging");
        assert_eq!(kept.variables.len(), 1);
    }

    #[test]
    fn importing_without_a_name_is_rejected() {
        let db = Database::open_in_memory().expect("db");
        assert_eq!(
            import(&db, "", &[]).expect_err("should fail").kind(),
            "invalidRequest"
        );
    }

    #[test]
    fn deleting_the_active_environment_clears_the_selection() {
        let db = Database::open_in_memory().expect("db");
        save(&db, &env("e1", "Staging")).expect("save");
        set_active(&db, Some("e1")).expect("set");
        delete(&db, "e1").expect("delete");
        assert_eq!(active_id(&db).expect("active"), None);
    }

    #[test]
    fn get_reports_not_found() {
        let db = Database::open_in_memory().expect("db");
        assert_eq!(
            get(&db, "nope").expect_err("should fail").kind(),
            "notFound"
        );
    }
}
