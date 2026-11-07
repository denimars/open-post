//! Postman-compatible import and export.
//!
//! Collections are written as [Postman Collection Format v2.1.0] and
//! environments as Postman environment files, so a collection exported here
//! opens in Postman, Insomnia, Bruno or Hoppscotch, and a file a teammate
//! exported from any of those can be imported back.
//!
//! open-post has no nested folders, so a folder tree is flattened into
//! `Folder/Sub/Request` on import and rebuilt from those names on export.
//!
//! [Postman Collection Format v2.1.0]: https://schema.postman.com/

pub mod export;
pub mod import;
pub mod schema;

use crate::error::AppResult;
use crate::models::{Collection, Environment};
use crate::storage::{collections, environments, Database};
use serde::Serialize;

/// A file ready to be written to disk: contents plus a suggested file name.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    pub file_name: String,
    pub contents: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CollectionImportReport {
    pub collection: Collection,
    pub imported_requests: usize,
    /// Everything that could not be mapped exactly, in plain language.
    pub warnings: Vec<String>,
    /// Set when the file carried collection-level variables and an environment
    /// was created from them.
    pub created_environment: Option<Environment>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentImportReport {
    pub environment: Environment,
    pub warnings: Vec<String>,
}

pub fn export_collection(db: &Database, collection_id: &str) -> AppResult<ExportFile> {
    let collection = collections::get(db, collection_id)?;
    Ok(ExportFile {
        file_name: file_name(&collection.name, "postman_collection"),
        contents: export::collection_json(&collection)?,
    })
}

pub fn export_environment(db: &Database, environment_id: &str) -> AppResult<ExportFile> {
    let environment = environments::get(db, environment_id)?;
    Ok(ExportFile {
        file_name: file_name(&environment.name, "postman_environment"),
        contents: export::environment_json(&environment)?,
    })
}

/// Imports a Postman collection as a new collection. Collection-level
/// variables become a new environment, since `{{var}}` would be unresolved
/// without them.
pub fn import_collection(db: &Database, json: &str) -> AppResult<CollectionImportReport> {
    let imported = import::parse_collection(json)?;
    let mut warnings = imported.warnings;

    let collection = collections::import(db, &imported.name, &imported.requests)?;

    let created_environment = if imported.variables.is_empty() {
        None
    } else {
        let environment = environments::import(
            db,
            &format!("{} variables", imported.name),
            &imported.variables,
        )?;
        warnings.push(format!(
            "{} collection variable(s) became the environment `{}`; select it to \
             resolve the {{{{variables}}}} in these requests",
            imported.variables.len(),
            environment.name
        ));
        Some(environment)
    };

    Ok(CollectionImportReport {
        imported_requests: collection.requests.len(),
        collection,
        warnings,
        created_environment,
    })
}

pub fn import_environment(db: &Database, json: &str) -> AppResult<EnvironmentImportReport> {
    let imported = import::parse_environment(json)?;
    let environment = environments::import(db, &imported.name, &imported.variables)?;
    Ok(EnvironmentImportReport {
        environment,
        warnings: imported.warnings,
    })
}

/// `My API` -> `my-api.postman_collection.json`.
fn file_name(name: &str, suffix: &str) -> String {
    let slug: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').replace("--", "-");
    let slug = if slug.is_empty() { "export" } else { &slug };
    format!("{slug}.{suffix}.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_postman_style_file_name() {
        assert_eq!(
            file_name("My API!", "postman_collection"),
            "my-api.postman_collection.json"
        );
        assert_eq!(
            file_name("  ", "postman_environment"),
            "export.postman_environment.json"
        );
    }
}
