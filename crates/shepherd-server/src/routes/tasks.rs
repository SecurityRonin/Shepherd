use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde_json::Value;
use shepherd_core::db::{models::CreateTask, queries};
use shepherd_core::events::{ServerEvent, TaskEvent};
use std::sync::Arc;

use crate::state::AppState;

#[tracing::instrument(skip(state))]
pub async fn list_tasks(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let tasks = queries::list_tasks(&db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::to_value(tasks).unwrap()))
}

#[tracing::instrument(skip(state, input))]
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateTask>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let task = queries::create_task(&db, &input).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    let _ = state.event_tx.send(ServerEvent::TaskCreated(TaskEvent {
        id: task.id,
        title: task.title.clone(),
        agent_id: task.agent_id.clone(),
        status: task.status.as_str().to_string(),
        branch: task.branch.clone(),
        repo_path: task.repo_path.clone(),
    }));
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(task).unwrap()),
    ))
}

#[tracing::instrument(skip(state))]
pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    queries::delete_task(&db, id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    let _ = state.event_tx.send(ServerEvent::TaskDeleted { id });
    Ok(Json(serde_json::json!({ "deleted": id })))
}
