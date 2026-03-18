use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Serialize)]
pub struct ClaudeSessionsResponse {
    pub sessions: Vec<String>,
}

#[derive(Deserialize)]
pub struct ResumeRequest {
    pub claude_session_id: Option<String>,
}

pub async fn list_claude_sessions(
    Path(task_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ClaudeSessionsResponse>, StatusCode> {
    let cwd = {
        let conn = state.db.lock().await;
        shepherd_core::db::queries::get_task(&conn, task_id)
            .map_err(|_| StatusCode::NOT_FOUND)?
            .repo_path
    };

    let project_dir = shepherd_core::iterm2::claude_project_dir(&cwd);
    let sessions = shepherd_core::iterm2::list_claude_sessions(
        project_dir.to_str().unwrap_or(""),
    );

    Ok(Json(ClaudeSessionsResponse { sessions }))
}

pub async fn resume_session(
    Path(task_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResumeRequest>,
) -> StatusCode {
    let iterm2_session_id = {
        let conn = state.db.lock().await;
        match shepherd_core::db::queries::get_task(&conn, task_id) {
            Ok(t) => t.iterm2_session_id,
            Err(_) => return StatusCode::NOT_FOUND,
        }
    };

    let Some(session_id) = iterm2_session_id else {
        return StatusCode::BAD_REQUEST;
    };

    tracing::info!(
        "Resume requested for task {task_id} (session {session_id}) with claude_session_id={:?}",
        body.claude_session_id
    );

    StatusCode::ACCEPTED
}

pub async fn fresh_session(
    Path(task_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    let iterm2_session_id = {
        let conn = state.db.lock().await;
        match shepherd_core::db::queries::get_task(&conn, task_id) {
            Ok(t) => t.iterm2_session_id,
            Err(_) => return StatusCode::NOT_FOUND,
        }
    };

    if iterm2_session_id.is_none() {
        return StatusCode::BAD_REQUEST;
    }

    StatusCode::ACCEPTED
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use shepherd_core::db;
    use shepherd_core::db::models::CreateTask;
    use tower::ServiceExt;

    async fn test_state() -> Arc<AppState> {
        let conn = db::open_memory().unwrap();
        let (event_tx, _) =
            tokio::sync::broadcast::channel::<shepherd_core::events::ServerEvent>(4);
        Arc::new(AppState {
            db: Arc::new(tokio::sync::Mutex::new(conn)),
            config: shepherd_core::config::load_config(None).unwrap(),
            adapters: shepherd_core::adapters::AdapterRegistry::new(),
            yolo: shepherd_core::yolo::YoloEngine::load(
                std::path::Path::new("/tmp/__nonexistent_shepherd_rules.yaml"),
            )
            .unwrap(),
            pty: shepherd_core::pty::PtyManager::new(
                1,
                shepherd_core::pty::sandbox::SandboxProfile::default(),
            ),
            event_tx,
            llm_provider: None,
            iterm2: None,
            cloud_client: None,
        })
    }

    #[tokio::test]
    async fn test_list_claude_sessions_not_found_task() {
        let state = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/sessions/9999/claude-sessions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_resume_session_not_found() {
        let state = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions/9999/resume")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_resume_session_non_iterm2_task_returns_bad_request() {
        let state = test_state().await;
        let task_id = {
            let conn = state.db.lock().await;
            shepherd_core::db::queries::create_task(&conn, &CreateTask {
                title: "regular task".into(),
                prompt: None,
                agent_id: "claude".into(),
                repo_path: None,
                isolation_mode: None,
                iterm2_session_id: None,
            })
            .unwrap()
            .id
        };
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/sessions/{task_id}/resume"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_resume_session_iterm2_task_returns_accepted() {
        let state = test_state().await;
        let task_id = {
            let conn = state.db.lock().await;
            shepherd_core::db::queries::create_task(&conn, &CreateTask {
                title: "iTerm2 task".into(),
                prompt: None,
                agent_id: "iterm2-adopted".into(),
                repo_path: Some("/tmp/proj".into()),
                isolation_mode: None,
                iterm2_session_id: Some("sess-resume-test".into()),
            })
            .unwrap()
            .id
        };
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/sessions/{task_id}/resume"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"claude_session_id":"abc-session"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_fresh_session_not_found() {
        let state = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/sessions/9999/fresh")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_fresh_session_non_iterm2_task_returns_bad_request() {
        let state = test_state().await;
        let task_id = {
            let conn = state.db.lock().await;
            shepherd_core::db::queries::create_task(&conn, &CreateTask {
                title: "regular task".into(),
                prompt: None,
                agent_id: "claude".into(),
                repo_path: None,
                isolation_mode: None,
                iterm2_session_id: None,
            })
            .unwrap()
            .id
        };
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/sessions/{task_id}/fresh"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_fresh_session_iterm2_task_returns_accepted() {
        let state = test_state().await;
        let task_id = {
            let conn = state.db.lock().await;
            shepherd_core::db::queries::create_task(&conn, &CreateTask {
                title: "iTerm2 task".into(),
                prompt: None,
                agent_id: "iterm2-adopted".into(),
                repo_path: Some("/tmp/proj".into()),
                isolation_mode: None,
                iterm2_session_id: Some("sess-fresh-test".into()),
            })
            .unwrap()
            .id
        };
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/api/sessions/{task_id}/fresh"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_list_claude_sessions_non_iterm2_task() {
        let state = test_state().await;
        let task_id = {
            let conn = state.db.lock().await;
            shepherd_core::db::queries::create_task(
                &conn,
                &CreateTask {
                    title: "test".into(),
                    prompt: None,
                    agent_id: "claude".into(),
                    repo_path: Some("/tmp/nonexistent-proj".into()),
                    isolation_mode: None,
                    iterm2_session_id: None,
                },
            )
            .unwrap()
            .id
        };
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/sessions/{task_id}/claude-sessions"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["sessions"], serde_json::json!([]));
    }
}
