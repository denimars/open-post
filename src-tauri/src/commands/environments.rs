use crate::error::AppResult;
use crate::models::Environment;
use crate::storage::environments;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn list_environments(state: State<'_, AppState>) -> AppResult<Vec<Environment>> {
    environments::list(&state.db)
}

#[tauri::command]
pub fn save_environment(
    state: State<'_, AppState>,
    environment: Environment,
) -> AppResult<Environment> {
    environments::save(&state.db, &environment)
}

#[tauri::command]
pub fn delete_environment(state: State<'_, AppState>, id: String) -> AppResult<()> {
    environments::delete(&state.db, &id)
}

#[tauri::command]
pub fn set_active_environment(state: State<'_, AppState>, id: Option<String>) -> AppResult<()> {
    environments::set_active(&state.db, id.as_deref())
}

#[tauri::command]
pub fn get_active_environment(state: State<'_, AppState>) -> AppResult<Option<String>> {
    environments::active_id(&state.db)
}
