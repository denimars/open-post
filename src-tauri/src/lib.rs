pub mod commands;
pub mod error;
pub mod http;
pub mod models;
pub mod postman;
pub mod storage;

use crate::error::{AppError, AppResult};
use crate::http::client::CancelRegistry;
use crate::storage::{db, Database};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tauri::Manager;

/// Shared application state: one HTTP client, one database, one cancel registry.
pub struct AppState {
    pub http: reqwest::Client,
    pub db: Database,
    pub cancels: Arc<CancelRegistry>,
    /// Problem found while starting up, surfaced in the UI once the window is
    /// there. `None` on a normal start.
    pub startup_warning: Option<String>,
}

/// Appends a line to `open-post.log` next to the database, and to stderr.
///
/// Launched from a desktop entry the app has no terminal, so without this file
/// a startup failure leaves no trace at all.
fn log_startup(dir: Option<&Path>, message: &str) {
    eprintln!("open-post: {message}");
    let Some(dir) = dir else { return };
    let stamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown time".into());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("open-post.log"))
    {
        let _ = writeln!(file, "{stamp} {message}");
    }
}

/// Builds the shared state. Storage problems degrade into a warning rather
/// than aborting startup, so the window is always created.
fn build_state(app: &tauri::App) -> AppResult<AppState> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::Storage(format!("could not locate the app data directory: {e}")));

    let (db, startup_warning, log_dir) = match dir {
        Ok(dir) => {
            let opened = db::open_or_recover(&dir.join("open-post.db"))?;
            (opened.db, opened.warning, Some(dir))
        }
        Err(e) => (
            Database::open_in_memory()?,
            Some(format!(
                "Running without storage: {e}. Collections, history and environments \
                 will not be saved."
            )),
            None,
        ),
    };

    if let Some(warning) = &startup_warning {
        log_startup(log_dir.as_deref(), warning);
    }
    Ok(AppState {
        http: crate::http::client::build_client()?,
        db,
        cancels: Arc::new(CancelRegistry::default()),
        startup_warning,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            app.manage(build_state(app)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::http::startup_warning,
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
        .build(tauri::generate_context!())
        .map(|app| {
            // Flush the WAL on the way out: an unclean exit otherwise leaves a
            // growing `-wal` beside a nearly empty database file.
            app.run(|handle, event| {
                if matches!(event, tauri::RunEvent::Exit) {
                    if let Some(state) = handle.try_state::<AppState>() {
                        state.db.checkpoint();
                    }
                }
            })
        });

    if let Err(e) = result {
        log_startup(None, &format!("could not start: {e}"));
        std::process::exit(1);
    }
}
