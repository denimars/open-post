//! Postman-compatible import/export commands.
//!
//! Export can either hand the JSON back to the frontend (for the clipboard or
//! a preview) or write it to disk itself. Import takes JSON the frontend read
//! with a file input, or a path to read here. Either way the webview never
//! needs the filesystem plugin.
//!
//! TODO: once `tauri-plugin-dialog` is available, pass the path the user picks
//! in a native "Save as…" / "Open" dialog to the `*_file` commands below; they
//! already accept one. Without it, an export without an explicit path lands in
//! the user's download directory.

use crate::error::{AppError, AppResult};
use crate::postman::{self, CollectionImportReport, EnvironmentImportReport, ExportFile};
use crate::AppState;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

/// An import file larger than this is refused rather than parsed; a Postman
/// collection is orders of magnitude smaller.
const MAX_IMPORT_BYTES: u64 = 32 * 1024 * 1024;

/// Where an export ended up, so the UI can show the full path.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SavedExport {
    pub path: String,
    pub file_name: String,
    pub bytes: usize,
}

#[tauri::command]
pub fn export_collection(
    state: State<'_, AppState>,
    collection_id: String,
) -> AppResult<ExportFile> {
    postman::export_collection(&state.db, &collection_id)
}

/// Writes the collection to `path`, or to the download directory when `path`
/// is omitted.
#[tauri::command]
pub fn export_collection_to_file(
    app: AppHandle,
    state: State<'_, AppState>,
    collection_id: String,
    path: Option<String>,
) -> AppResult<SavedExport> {
    let file = postman::export_collection(&state.db, &collection_id)?;
    write(&app, &file, path.as_deref())
}

#[tauri::command]
pub fn import_collection(
    state: State<'_, AppState>,
    json: String,
) -> AppResult<CollectionImportReport> {
    postman::import_collection(&state.db, &json)
}

#[tauri::command]
pub fn import_collection_from_file(
    state: State<'_, AppState>,
    path: String,
) -> AppResult<CollectionImportReport> {
    postman::import_collection(&state.db, &read(&path)?)
}

#[tauri::command]
pub fn export_environment(
    state: State<'_, AppState>,
    environment_id: String,
) -> AppResult<ExportFile> {
    postman::export_environment(&state.db, &environment_id)
}

#[tauri::command]
pub fn export_environment_to_file(
    app: AppHandle,
    state: State<'_, AppState>,
    environment_id: String,
    path: Option<String>,
) -> AppResult<SavedExport> {
    let file = postman::export_environment(&state.db, &environment_id)?;
    write(&app, &file, path.as_deref())
}

#[tauri::command]
pub fn import_environment(
    state: State<'_, AppState>,
    json: String,
) -> AppResult<EnvironmentImportReport> {
    postman::import_environment(&state.db, &json)
}

#[tauri::command]
pub fn import_environment_from_file(
    state: State<'_, AppState>,
    path: String,
) -> AppResult<EnvironmentImportReport> {
    postman::import_environment(&state.db, &read(&path)?)
}

/// Resolves the target path and writes the file.
///
/// `path` may be a full file path or an existing directory; without one the
/// download directory is used, falling back to the home directory.
fn write(app: &AppHandle, file: &ExportFile, path: Option<&str>) -> AppResult<SavedExport> {
    let target = match path {
        Some(path) if !path.trim().is_empty() => {
            let path = PathBuf::from(path.trim());
            if path.is_dir() {
                path.join(&file.file_name)
            } else {
                path
            }
        }
        _ => default_directory(app)?.join(&file.file_name),
    };

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::Storage(format!("could not create {}: {e}", parent.display()))
        })?;
    }
    std::fs::write(&target, &file.contents)
        .map_err(|e| AppError::Storage(format!("could not write {}: {e}", target.display())))?;

    Ok(SavedExport {
        path: target.to_string_lossy().into_owned(),
        file_name: file.file_name.clone(),
        bytes: file.contents.len(),
    })
}

fn default_directory(app: &AppHandle) -> AppResult<PathBuf> {
    let path = app.path();
    path.download_dir()
        .or_else(|_| path.home_dir())
        .map_err(|e| AppError::Storage(format!("could not locate a directory to save in: {e}")))
}

fn read(path: &str) -> AppResult<String> {
    let path = Path::new(path);
    let size = std::fs::metadata(path)
        .map_err(|e| AppError::Storage(format!("could not open {}: {e}", path.display())))?
        .len();
    if size > MAX_IMPORT_BYTES {
        return Err(AppError::InvalidRequest(format!(
            "{} is {} MB; that is too large for a collection file",
            path.display(),
            size / 1024 / 1024
        )));
    }
    std::fs::read_to_string(path)
        .map_err(|e| AppError::Storage(format!("could not read {}: {e}", path.display())))
}
