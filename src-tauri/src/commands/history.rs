use crate::error::AppResult;
use crate::models::HistoryEntry;
use crate::storage::history;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn list_history(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> AppResult<Vec<HistoryEntry>> {
    history::list(&state.db, limit.unwrap_or(100).min(500))
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> AppResult<()> {
    history::clear(&state.db)
}
