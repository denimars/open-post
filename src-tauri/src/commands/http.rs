//! HTTP commands. These stay thin: resolve state, delegate, record history.

use crate::error::AppResult;
use crate::http::client::{self, CancelRegistry};
use crate::models::{HttpRequest, HttpResponse, KeyValue};
use crate::storage::{environments, history, Database};
use crate::AppState;
use std::sync::Arc;
use tauri::State;

fn variables_for(db: &Database, environment_id: Option<&str>) -> Vec<KeyValue> {
    let id = match environment_id {
        Some(id) => Some(id.to_string()),
        None => environments::active_id(db).ok().flatten(),
    };
    id.and_then(|id| environments::get(db, &id).ok())
        .map(|env| env.variables)
        .unwrap_or_default()
}

/// A storage problem found at startup, or `None` on a normal start. The UI
/// shows it in a banner.
#[tauri::command]
pub fn startup_warning(state: State<'_, AppState>) -> Option<String> {
    state.startup_warning.clone()
}

#[tauri::command]
pub async fn send_request(
    state: State<'_, AppState>,
    request: HttpRequest,
    environment_id: Option<String>,
) -> AppResult<HttpResponse> {
    let variables = variables_for(&state.db, environment_id.as_deref());

    let registry: Arc<CancelRegistry> = Arc::clone(&state.cancels);
    let token = registry.register(&request.id);

    let (result, resolved) = client::send(&state.http, &request, &variables, token).await;
    registry.finish(&request.id);

    let (status, duration) = match &result {
        Ok(response) => (Some(response.status), Some(response.duration_ms)),
        Err(_) => (None, None),
    };
    // A failed send is still worth remembering; a storage failure must not
    // hide the response.
    let _ = history::record(&state.db, &resolved, status, duration);

    result
}

#[tauri::command]
pub fn cancel_request(state: State<'_, AppState>, request_id: String) -> bool {
    state.cancels.cancel(&request_id)
}
