use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde_json::Value;
use std::sync::Arc;

use crate::state::AppState;

/// GET /api/replay/task/:taskId — returns event timeline for a task.
/// Delegates to the existing `shepherd_core::replay::get_timeline()`.
/// Returns empty array (not 404) if task has no events.
#[tracing::instrument(skip(state))]
pub async fn replay_events(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let events = shepherd_core::replay::get_timeline(&db, task_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::to_value(events).unwrap()))
}
