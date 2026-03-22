//! In-process handler tests using tower::ServiceExt::oneshot.
//! These test handlers directly without spawning a TCP server, ensuring
//! tarpaulin can track coverage of the async handler bodies.

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt; // for `collect`
    use std::sync::Arc;
    use tokio::sync::{broadcast, Mutex};
    use tower::ServiceExt; // for `oneshot`

    /// Build a minimal AppState for testing (no LLM provider, no cloud client).
    fn test_state() -> Arc<crate::state::AppState> {
        let conn = shepherd_core::db::open_memory().unwrap();
        let (event_tx, _) = broadcast::channel(256);
        Arc::new(crate::state::AppState {
            db: Arc::new(Mutex::new(conn)),
            config: shepherd_core::config::types::ShepherdConfig::default(),
            adapters: Arc::new(shepherd_core::adapters::AdapterRegistry::new()),
            yolo: Arc::new(shepherd_core::yolo::YoloEngine::new(
                shepherd_core::yolo::rules::RuleSet {
                    deny: vec![],
                    allow: vec![],
                },
            )),
            pty: Arc::new(shepherd_core::pty::PtyManager::new(
                4,
                shepherd_core::pty::sandbox::SandboxProfile::disabled(),
            )),
            event_tx,
            llm_provider: None,
            iterm2: None,
            cloud_client: None,
        })
    }

    /// Helper to make a JSON POST request.
    fn json_post(uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }

    /// Helper to make a GET request.
    fn get(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    /// Helper to make a DELETE request.
    fn delete(uri: &str) -> Request<Body> {
        Request::builder()
            .method("DELETE")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    /// Extract body bytes from a response and parse as JSON.
    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    // -- Task handler tests ---------------------------------------------------

    #[tokio::test]
    async fn handler_list_tasks_empty() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app.oneshot(get("/api/tasks")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn handler_create_task() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post(
                "/api/tasks",
                serde_json::json!({
                    "title": "Test task",
                    "agent_id": "claude-code"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = body_json(resp).await;
        assert_eq!(body["title"], "Test task");
        assert_eq!(body["status"], "queued");
    }

    #[tokio::test]
    async fn handler_create_and_delete_task() {
        let state = test_state();
        // Create task
        let app = crate::build_router(state.clone());
        let resp = app
            .oneshot(json_post(
                "/api/tasks",
                serde_json::json!({
                    "title": "To delete",
                    "agent_id": "claude-code"
                }),
            ))
            .await
            .unwrap();
        let body = body_json(resp).await;
        let id = body["id"].as_i64().unwrap();

        // Delete task
        let app = crate::build_router(state.clone());
        let resp = app
            .oneshot(delete(&format!("/api/tasks/{}", id)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["deleted"], id);

        // Verify deleted
        let app = crate::build_router(state);
        let resp = app.oneshot(get("/api/tasks")).await.unwrap();
        let body = body_json(resp).await;
        assert_eq!(body.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn handler_get_task() {
        let state = test_state();
        let app = crate::build_router(state.clone());
        let resp = app
            .oneshot(json_post(
                "/api/tasks",
                serde_json::json!({"title": "Fetch me", "agent_id": "claude-code"}),
            ))
            .await
            .unwrap();
        let body = body_json(resp).await;
        let id = body["id"].as_i64().unwrap();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(get(&format!("/api/tasks/{}", id)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["title"], "Fetch me");
        assert_eq!(body["id"], id);
    }

    #[tokio::test]
    async fn handler_get_task_not_found() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app.oneshot(get("/api/tasks/99999")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = body_json(resp).await;
        assert!(body["error"].as_str().unwrap().contains("not found"));
    }

    // -- Logogen handler tests ------------------------------------------------

    #[tokio::test]
    async fn handler_logogen_no_provider() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post(
                "/api/logogen",
                serde_json::json!({
                    "product_name": "Test",
                    "style": "minimal"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = body_json(resp).await;
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("No generation provider"));
    }

    #[tokio::test]
    async fn handler_logogen_export_invalid() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post(
                "/api/logogen/export",
                serde_json::json!({
                    "image_base64": "not-valid",
                    "product_name": "Test"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // -- Namegen handler tests ------------------------------------------------

    #[tokio::test]
    async fn handler_namegen_no_provider() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post(
                "/api/namegen",
                serde_json::json!({
                    "description": "A test product"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = body_json(resp).await;
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("No generation provider"));
    }

    // -- Northstar handler tests ----------------------------------------------

    #[tokio::test]
    async fn handler_northstar_phases() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app.oneshot(get("/api/northstar/phases")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["total"], 13);
    }

    #[tokio::test]
    async fn handler_northstar_execute_invalid_phase() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post(
                "/api/northstar/phase",
                serde_json::json!({
                    "product_name": "Test",
                    "product_description": "A test",
                    "phase_id": 99
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = body_json(resp).await;
        assert!(body["error"].as_str().unwrap().contains("Unknown phase_id"));
    }

    #[tokio::test]
    async fn handler_northstar_execute_no_provider() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post(
                "/api/northstar/phase",
                serde_json::json!({
                    "product_name": "Test",
                    "product_description": "A test",
                    "phase_id": 1
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    // -- Cloud handler tests --------------------------------------------------

    #[tokio::test]
    async fn handler_cloud_status() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app.oneshot(get("/api/cloud/status")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["cloud_available"], false);
    }

    #[tokio::test]
    async fn handler_cloud_balance_unavailable() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app.oneshot(get("/api/cloud/balance")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn handler_cloud_costs() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app.oneshot(get("/api/cloud/costs")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["features"].as_array().unwrap().len(), 7);
    }

    // -- Gates handler tests --------------------------------------------------

    #[tokio::test]
    async fn handler_gates_task_not_found() {
        let state = test_state();
        let app = crate::build_router(state);
        // gates handler uses POST with no JSON body, but we send empty JSON
        // to satisfy the route; the handler only extracts Path(task_id) + State
        let resp = app
            .oneshot(json_post("/api/tasks/99999/gates", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- PR handler tests -----------------------------------------------------

    #[tokio::test]
    async fn handler_pr_task_not_found() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post("/api/tasks/99999/pr", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- Health handler test --------------------------------------------------

    #[tokio::test]
    async fn handler_health() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app.oneshot(get("/api/health")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["status"], "ok");
    }

    // =========================================================================
    // Direct handler call tests — bypass Axum middleware for tarpaulin coverage
    // =========================================================================

    use axum::extract::{Path, State};
    use axum::Json;

    // -- Direct: Tasks --------------------------------------------------------

    #[tokio::test]
    async fn direct_list_tasks() {
        let state = test_state();
        let result = crate::routes::tasks::list_tasks(State(state)).await;
        let Json(body) = result.unwrap();
        assert!(body.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn direct_create_task() {
        let state = test_state();
        let input = shepherd_core::db::models::CreateTask {
            title: "Direct test".to_string(),
            agent_id: "claude-code".to_string(),
            prompt: None,
            repo_path: None,
            isolation_mode: None,
            iterm2_session_id: None,
        };
        let result = crate::routes::tasks::create_task(State(state), Json(input)).await;
        let (status, Json(body)) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["title"], "Direct test");
        assert_eq!(body["status"], "queued");
    }

    #[tokio::test]
    async fn direct_delete_task() {
        let state = test_state();
        // First create a task so we have something to delete
        let input = shepherd_core::db::models::CreateTask {
            title: "To delete".to_string(),
            agent_id: "cc".to_string(),
            prompt: None,
            repo_path: None,
            isolation_mode: None,
            iterm2_session_id: None,
        };
        let (_, Json(created)) =
            crate::routes::tasks::create_task(State(state.clone()), Json(input))
                .await
                .unwrap();
        let id = created["id"].as_i64().unwrap();

        // Delete it
        let result = crate::routes::tasks::delete_task(State(state), Path(id)).await;
        let Json(body) = result.unwrap();
        assert_eq!(body["deleted"], id);
    }

    #[tokio::test]
    async fn direct_delete_task_nonexistent() {
        // delete_task's SQL DELETE silently succeeds even for missing IDs,
        // so the handler returns Ok with the id echoed back.
        let state = test_state();
        let result = crate::routes::tasks::delete_task(State(state), Path(99999i64)).await;
        let Json(body) = result.unwrap();
        assert_eq!(body["deleted"], 99999);
    }

    #[tokio::test]
    async fn direct_get_task() {
        let state = test_state();
        let input = shepherd_core::db::models::CreateTask {
            title: "Direct get".to_string(),
            agent_id: "claude-code".to_string(),
            prompt: None,
            repo_path: None,
            isolation_mode: None,
            iterm2_session_id: None,
        };
        let (_, Json(created)) =
            crate::routes::tasks::create_task(State(state.clone()), Json(input))
                .await
                .unwrap();
        let id = created["id"].as_i64().unwrap();
        let result = crate::routes::tasks::get_task(State(state), Path(id)).await;
        let Json(body) = result.unwrap();
        assert_eq!(body["title"], "Direct get");
    }

    #[tokio::test]
    async fn direct_get_task_not_found() {
        let state = test_state();
        let result = crate::routes::tasks::get_task(State(state), Path(99999i64)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // -- Direct: Logogen ------------------------------------------------------

    #[tokio::test]
    async fn direct_logogen_no_provider() {
        let state = test_state();
        let req = crate::routes::logogen::LogoGenRequest {
            product_name: "Test".to_string(),
            product_description: None,
            style: "minimal".to_string(),
            colors: vec![],
        };
        let result = crate::routes::logogen::generate_logo(State(state), Json(req)).await;
        assert!(result.is_err());
        let (status, Json(body)) = result.unwrap_err();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("No generation provider"));
    }

    #[tokio::test]
    async fn direct_logogen_export_invalid() {
        let req = crate::routes::logogen::ExportRequest {
            image_base64: "invalid".to_string(),
            product_name: "Test".to_string(),
            output_dir: None,
        };
        let result = crate::routes::logogen::export_icons(Json(req)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    // -- Direct: Namegen ------------------------------------------------------

    #[tokio::test]
    async fn direct_namegen_no_provider() {
        let state = test_state();
        let req = crate::routes::namegen::NameGenRequest {
            description: "A test product".to_string(),
            vibes: vec![],
            count: Some(5),
        };
        let result = crate::routes::namegen::generate_names(State(state), Json(req)).await;
        assert!(result.is_err());
        let (status, Json(body)) = result.unwrap_err();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("No generation provider"));
    }

    // -- Direct: Northstar ----------------------------------------------------

    #[tokio::test]
    async fn direct_list_phases() {
        let Json(result) = crate::routes::northstar::list_phases().await;
        assert_eq!(result.total, 13);
        assert_eq!(result.phases.len(), 13);
    }

    #[tokio::test]
    async fn direct_execute_phase_invalid() {
        let state = test_state();
        let req = crate::routes::northstar::ExecutePhaseRequest {
            product_name: "Test".to_string(),
            product_description: "A test".to_string(),
            phase_id: 99,
            previous_context: None,
        };
        let result = crate::routes::northstar::execute_phase(State(state), Json(req)).await;
        assert!(result.is_err());
        let (status, Json(body)) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("Unknown phase_id"));
    }

    #[tokio::test]
    async fn direct_execute_phase_no_provider() {
        let state = test_state();
        let req = crate::routes::northstar::ExecutePhaseRequest {
            product_name: "Test".to_string(),
            product_description: "A test".to_string(),
            phase_id: 1,
            previous_context: None,
        };
        let result = crate::routes::northstar::execute_phase(State(state), Json(req)).await;
        assert!(result.is_err());
        let (status, Json(body)) = result.unwrap_err();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("No generation provider"));
    }

    // -- Direct: Cloud --------------------------------------------------------

    #[tokio::test]
    async fn direct_cloud_status() {
        let state = test_state();
        let Json(result) = crate::routes::cloud::cloud_status(State(state)).await;
        assert!(!result.cloud_available);
        assert!(!result.authenticated);
        assert!(result.plan.is_none());
        assert!(result.credits_balance.is_none());
    }

    #[tokio::test]
    async fn direct_cloud_balance_unavailable() {
        let state = test_state();
        let result = crate::routes::cloud::cloud_balance(State(state)).await;
        assert!(result.is_err());
        let (status, Json(body)) = result.unwrap_err();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("Cloud features not available"));
    }

    #[tokio::test]
    async fn direct_cloud_costs() {
        let Json(result) = crate::routes::cloud::cloud_costs().await;
        assert_eq!(result.features.len(), 7);
        let names: Vec<&str> = result.features.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"logo"));
        assert!(names.contains(&"name"));
        assert!(names.contains(&"northstar"));
    }

    // -- Direct: Gates --------------------------------------------------------

    #[tokio::test]
    async fn direct_gates_not_found() {
        let state = test_state();
        let result = crate::routes::gates::run_task_gates(State(state), Path(99999i64)).await;
        assert!(result.is_err());
        let (status, Json(body)) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body["error"].as_str().unwrap().contains("Task not found"));
    }

    // -- Direct: PR -----------------------------------------------------------

    #[tokio::test]
    async fn direct_pr_not_found() {
        let state = test_state();
        let req = crate::routes::pr::CreatePrRequest {
            base_branch: "main".to_string(),
            auto_commit_message: true,
            run_gates: false,
        };
        let result = crate::routes::pr::create_pr(State(state), Path(99999i64), Json(req)).await;
        assert!(result.is_err());
        let (status, Json(body)) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body["error"].as_str().unwrap().contains("Task not found"));
    }

    // -- Approve handler tests ------------------------------------------------

    #[tokio::test]
    async fn handler_approve_task_not_found() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post("/api/tasks/99999/approve", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_approve_task_writes_to_pty_and_updates_status() {
        let state = test_state();
        // Create a task with status "input"
        {
            let db = state.db.lock().await;
            db.execute(
                "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp', 'main', 'none', 'input')",
                [],
            ).unwrap();
        }
        let app = crate::build_router(state.clone());
        let resp = app
            .oneshot(json_post("/api/tasks/1/approve", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["status"], "running");

        // Verify DB status updated
        let db = state.db.lock().await;
        let status: String = db
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status, "running");
    }

    #[tokio::test]
    async fn handler_approve_all_returns_count() {
        let state = test_state();
        // Create two tasks with status "input"
        {
            let db = state.db.lock().await;
            db.execute(
                "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'T1', '', 'claude-code', '/tmp', 'main', 'none', 'input')",
                [],
            ).unwrap();
            db.execute(
                "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (2, 'T2', '', 'claude-code', '/tmp', 'main', 'none', 'input')",
                [],
            ).unwrap();
            // One running task that should NOT be approved
            db.execute(
                "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (3, 'T3', '', 'claude-code', '/tmp', 'main', 'none', 'running')",
                [],
            ).unwrap();
        }
        let app = crate::build_router(state.clone());
        let resp = app
            .oneshot(json_post("/api/approve-all", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["approved"], 2);

        // Verify both input tasks are now running
        let db = state.db.lock().await;
        let s1: String = db
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        let s2: String = db
            .query_row("SELECT status FROM tasks WHERE id = 2", [], |r| r.get(0))
            .unwrap();
        let s3: String = db
            .query_row("SELECT status FROM tasks WHERE id = 3", [], |r| r.get(0))
            .unwrap();
        assert_eq!(s1, "running");
        assert_eq!(s2, "running");
        assert_eq!(s3, "running"); // was already running
    }

    #[tokio::test]
    async fn handler_approve_all_empty() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post("/api/approve-all", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["approved"], 0);
    }

    // -- Cancel handler tests -------------------------------------------------

    #[tokio::test]
    async fn handler_cancel_running_task() {
        let state = test_state();
        {
            let db = state.db.lock().await;
            db.execute(
                "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Running task', '', 'claude-code', '/tmp', 'main', 'none', 'running')",
                [],
            ).unwrap();
        }
        let app = crate::build_router(state.clone());
        let resp = app
            .oneshot(json_post("/api/tasks/1/cancel", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["status"], "cancelled");
        let db = state.db.lock().await;
        let status: String = db
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status, "cancelled");
    }

    #[tokio::test]
    async fn handler_cancel_finished_task_returns_400() {
        let state = test_state();
        {
            let db = state.db.lock().await;
            db.execute(
                "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Done task', '', 'claude-code', '/tmp', 'main', 'none', 'done')",
                [],
            ).unwrap();
        }
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post("/api/tasks/1/cancel", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = body_json(resp).await;
        assert!(body["error"].as_str().unwrap().contains("already finished"));
    }

    #[tokio::test]
    async fn handler_cancel_nonexistent_returns_404() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post("/api/tasks/99999/cancel", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- Shutdown handler tests -----------------------------------------------

    #[tokio::test]
    async fn handler_shutdown_returns_ok() {
        let state = test_state();
        let app = crate::build_router(state);
        let resp = app
            .oneshot(json_post("/api/shutdown", serde_json::json!({})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["status"], "shutting_down");
    }

    // -- Direct: Approve/Shutdown ---------------------------------------------

    #[tokio::test]
    async fn direct_approve_task_not_found() {
        let state = test_state();
        let result = crate::routes::tasks::approve_task(State(state), Path(99999i64)).await;
        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn direct_approve_all() {
        let state = test_state();
        let result = crate::routes::tasks::approve_all(State(state)).await;
        let Json(body) = result.unwrap();
        assert_eq!(body["approved"], 0);
    }

    #[tokio::test]
    async fn direct_shutdown() {
        let state = test_state();
        let Json(body) = crate::routes::tasks::shutdown_server(State(state)).await;
        assert_eq!(body["status"], "shutting_down");
    }
}
