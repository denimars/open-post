use crate::error::AppResult;
use crate::models::{Collection, HttpRequest, SavedRequest};
use crate::storage::collections;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn list_collections(state: State<'_, AppState>) -> AppResult<Vec<Collection>> {
    collections::list(&state.db)
}

#[tauri::command]
pub fn create_collection(state: State<'_, AppState>, name: String) -> AppResult<Collection> {
    collections::create(&state.db, &name)
}

#[tauri::command]
pub fn rename_collection(state: State<'_, AppState>, id: String, name: String) -> AppResult<()> {
    collections::rename(&state.db, &id, &name)
}

#[tauri::command]
pub fn delete_collection(state: State<'_, AppState>, id: String) -> AppResult<()> {
    collections::delete(&state.db, &id)
}

#[tauri::command]
pub fn save_request(
    state: State<'_, AppState>,
    collection_id: String,
    name: String,
    request: HttpRequest,
) -> AppResult<SavedRequest> {
    collections::save_request(&state.db, &collection_id, &name, &request)
}

#[tauri::command]
pub fn delete_request(state: State<'_, AppState>, id: String) -> AppResult<()> {
    collections::delete_request(&state.db, &id)
}
