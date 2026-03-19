use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde_json::Value;
use shepherd_core::db::models::TaskStatus;
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
        iterm2_session_id: task.iterm2_session_id.clone(),
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

/// Approve a pending permission for a task — sends the approve keystroke to PTY
/// and transitions task status from "input" back to "running".
#[tracing::instrument(skip(state))]
pub async fn approve_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let task = queries::get_task(&db, id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Task not found: {}", e) })),
        )
    })?;

    let approve_str = state
        .adapters
        .get(&task.agent_id)
        .map(|a| a.permissions.approve.clone())
        .unwrap_or_else(|| "y\n".into());

    queries::update_task_status(&db, id, &TaskStatus::Running).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    drop(db);

    let _ = state.pty.write_to(id, &approve_str).await;
    let _ = state.event_tx.send(ServerEvent::TaskUpdated(TaskEvent {
        id: task.id,
        title: task.title,
        agent_id: task.agent_id,
        status: "running".into(),
        branch: task.branch,
        repo_path: task.repo_path,
        iterm2_session_id: task.iterm2_session_id,
    }));

    Ok(Json(serde_json::json!({ "id": id, "status": "running" })))
}

/// Approve all tasks currently in "input" status.
#[tracing::instrument(skip(state))]
pub async fn approve_all(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Phase 1: Hold lock, update DB statuses, collect data for PTY writes
    let pending = {
        let db = state.db.lock().await;
        let tasks = queries::list_tasks(&db).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

        let pending: Vec<_> = tasks
            .into_iter()
            .filter(|t| t.status == TaskStatus::Input)
            .collect();

        for task in &pending {
            let _ = queries::update_task_status(&db, task.id, &TaskStatus::Running);
        }
        pending
        // db lock dropped here
    };

    let count = pending.len();

    // Phase 2: No lock held — send PTY writes and events
    for task in &pending {
        let approve_str = state
            .adapters
            .get(&task.agent_id)
            .map(|a| a.permissions.approve.clone())
            .unwrap_or_else(|| "y\n".into());
        let _ = state.pty.write_to(task.id, &approve_str).await;
        let _ = state.event_tx.send(ServerEvent::TaskUpdated(TaskEvent {
            id: task.id,
            title: task.title.clone(),
            agent_id: task.agent_id.clone(),
            status: "running".into(),
            branch: task.branch.clone(),
            repo_path: task.repo_path.clone(),
            iterm2_session_id: task.iterm2_session_id.clone(),
        }));
    }

    Ok(Json(serde_json::json!({ "approved": count })))
}

/// Graceful server shutdown — kills all agents and signals exit.
#[tracing::instrument(skip(state))]
pub async fn shutdown_server(State(state): State<Arc<AppState>>) -> Json<Value> {
    state
        .pty
        .shutdown_all(std::time::Duration::from_secs(5))
        .await;
    Json(serde_json::json!({ "status": "shutting_down" }))
}

#[cfg(test)]
mod tests {
    use shepherd_core::db::models::CreateTask;

    #[test]
    fn create_task_deserialize_minimal() {
        let json = r#"{
            "title": "Fix login bug",
            "agent_id": "claude-code"
        }"#;
        let task: CreateTask = serde_json::from_str(json).unwrap();
        assert_eq!(task.title, "Fix login bug");
        assert_eq!(task.agent_id, "claude-code");
        assert!(task.prompt.is_none());
        assert!(task.repo_path.is_none());
        assert!(task.isolation_mode.is_none());
        assert!(task.iterm2_session_id.is_none());
    }

    #[test]
    fn create_task_deserialize_full() {
        let json = r#"{
            "title": "Add feature",
            "prompt": "Implement the new login flow",
            "agent_id": "claude-code",
            "repo_path": "/tmp/repo",
            "isolation_mode": "worktree",
            "iterm2_session_id": "sess-123"
        }"#;
        let task: CreateTask = serde_json::from_str(json).unwrap();
        assert_eq!(task.title, "Add feature");
        assert_eq!(
            task.prompt,
            Some("Implement the new login flow".to_string())
        );
        assert_eq!(task.repo_path, Some("/tmp/repo".to_string()));
        assert_eq!(task.isolation_mode, Some("worktree".to_string()));
        assert_eq!(task.iterm2_session_id, Some("sess-123".to_string()));
    }

    #[test]
    fn create_task_serialize_roundtrip() {
        let task = CreateTask {
            title: "Test task".to_string(),
            prompt: Some("Do something".to_string()),
            agent_id: "agent-1".to_string(),
            repo_path: None,
            isolation_mode: None,
            iterm2_session_id: None,
        };
        let json_str = serde_json::to_string(&task).unwrap();
        let parsed: CreateTask = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.title, task.title);
        assert_eq!(parsed.prompt, task.prompt);
        assert_eq!(parsed.agent_id, task.agent_id);
    }
}
