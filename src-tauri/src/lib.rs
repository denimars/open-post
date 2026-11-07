pub mod commands;
pub mod error;
pub mod http;
pub mod models;
pub mod postman;
pub mod storage;

use crate::error::AppError;
use crate::http::client::CancelRegistry;
use crate::storage::Database;
use std::sync::Arc;
use tauri::Manager;

/// Shared application state: one HTTP client, one database, one cancel registry.
pub struct AppState {
    pub http: reqwest::Client,
    pub db: Database,
    pub cancels: Arc<CancelRegistry>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| {
                AppError::Storage(format!("could not locate the app data directory: {e}"))
            })?;
            let db = Database::open(&dir.join("open-post.db"))?;
            let http = crate::http::client::build_client()?;

            app.manage(AppState {
                http,
                db,
                cancels: Arc::new(CancelRegistry::default()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::http::send_request,
            commands::http::cancel_request,
            commands::history::list_history,
            commands::history::clear_history,
            commands::collections::list_collections,
            commands::collections::create_collection,
            commands::collections::rename_collection,
            commands::collections::delete_collection,
            commands::collections::save_request,
            commands::collections::delete_request,
            commands::transfer::export_collection,
            commands::transfer::export_collection_to_file,
            commands::transfer::import_collection,
            commands::transfer::import_collection_from_file,
            commands::transfer::export_environment,
            commands::transfer::export_environment_to_file,
            commands::transfer::import_environment,
            commands::transfer::import_environment_from_file,
            commands::environments::list_environments,
            commands::environments::save_environment,
            commands::environments::delete_environment,
            commands::environments::set_active_environment,
            commands::environments::get_active_environment,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the open-post application");
}
