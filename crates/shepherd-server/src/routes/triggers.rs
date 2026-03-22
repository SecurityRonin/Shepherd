use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use rusqlite::params;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct TriggerCheckRequest {
    project_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct TriggerDismissRequest {
    trigger_id: String,
    project_dir: String,
}

/// POST /api/triggers/check
#[tracing::instrument(skip(state))]
pub async fn check_triggers(
    State(state): State<Arc<AppState>>,
    Json(input): Json<TriggerCheckRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let project_path = Path::new(&input.project_dir);
    let metadata = std::fs::metadata(project_path).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Directory does not exist" })),
        )
    })?;
    if !metadata.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Path is not a directory" })),
        ));
    }
    if !project_path.join(".git").exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Not a git repository" })),
        ));
    }
    let dismissed = {
        let db = state.db.lock().await;
        let mut stmt = db
            .prepare("SELECT trigger_id FROM trigger_dismissals WHERE project_dir = ?1")
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;
        let ids: Vec<String> = stmt
            .query_map(params![input.project_dir], |row| row.get(0))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?
            .filter_map(|r| r.ok())
            .collect();
        ids
    };
    let suggestions = shepherd_core::triggers::check_triggers(project_path, &dismissed);
    Ok(Json(serde_json::to_value(suggestions).unwrap()))
}

/// POST /api/triggers/dismiss
#[tracing::instrument(skip(state))]
pub async fn dismiss_trigger(
    State(state): State<Arc<AppState>>,
    Json(input): Json<TriggerDismissRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    db.execute(
        "INSERT OR IGNORE INTO trigger_dismissals (trigger_id, project_dir) VALUES (?1, ?2)",
        params![input.trigger_id, input.project_dir],
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::json!({ "success": true })))
}
