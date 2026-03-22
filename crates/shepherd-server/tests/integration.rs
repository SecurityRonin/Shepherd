//! Full HTTP integration tests -- spin up a real server on an ephemeral port
//! and exercise the entire request->DB->response pipeline.

use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

/// Start the server on an ephemeral port and return (base_url, JoinHandle).
/// Uses an in-memory SQLite database and minimal state (no cloud, no iTerm2).
async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let port = portpicker::pick_unused_port().expect("No free port");
    let url = format!("http://127.0.0.1:{port}");

    let handle = tokio::spawn(async move {
        let cfg = shepherd_core::config::types::ShepherdConfig {
            port,
            ..Default::default()
        };
        let conn = shepherd_core::db::open_memory().unwrap();
        let adapters = std::sync::Arc::new(shepherd_core::adapters::AdapterRegistry::new());
        let yolo = std::sync::Arc::new(shepherd_core::yolo::YoloEngine::new(
            shepherd_core::yolo::rules::RuleSet {
                deny: vec![],
                allow: vec![],
            },
        ));
        let pty = std::sync::Arc::new(shepherd_core::pty::PtyManager::new(
            cfg.max_agents,
            shepherd_core::pty::sandbox::SandboxProfile::disabled(),
        ));
        let (event_tx, _) = tokio::sync::broadcast::channel(256);

        let state = std::sync::Arc::new(shepherd_server::state::AppState {
            db: std::sync::Arc::new(tokio::sync::Mutex::new(conn)),
            config: cfg,
            adapters,
            yolo,
            pty,
            event_tx,
            llm_provider: None,
            iterm2: None,
            cloud_client: None,
        });

        let app = shepherd_server::build_router(state);

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for server to be ready
    let client = Client::new();
    for _ in 0..20 {
        if client
            .get(&format!("{}/api/health", url))
            .send()
            .await
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    (url, handle)
}

#[tokio::test]
async fn integration_health_check() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    let resp = client
        .get(&format!("{}/api/health", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn integration_task_lifecycle() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    // Create task
    let resp = client
        .post(&format!("{}/api/tasks", base))
        .json(&json!({ "title": "Integration test", "agent_id": "claude-code" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let task: Value = resp.json().await.unwrap();
    let id = task["id"].as_i64().unwrap();
    assert_eq!(task["status"], "queued");

    // Fetch task
    let resp = client
        .get(&format!("{}/api/tasks/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let fetched: Value = resp.json().await.unwrap();
    assert_eq!(fetched["title"], "Integration test");

    // Cancel task
    let resp = client
        .post(&format!("{}/api/tasks/{}/cancel", base, id))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "cancelled");

    // Verify cancelled -- can't cancel again
    let resp = client
        .post(&format!("{}/api/tasks/{}/cancel", base, id))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Delete task
    let resp = client
        .delete(&format!("{}/api/tasks/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn integration_plugins_detected() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    let resp = client
        .get(&format!("{}/api/plugins/detected", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["detected"].is_array());
}

#[tokio::test]
async fn integration_replay_empty() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    // Create a task first
    let resp = client
        .post(&format!("{}/api/tasks", base))
        .json(&json!({ "title": "Replay test", "agent_id": "claude-code" }))
        .send()
        .await
        .unwrap();
    let task: Value = resp.json().await.unwrap();
    let id = task["id"].as_i64().unwrap();

    let resp = client
        .get(&format!("{}/api/replay/task/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn integration_automation_rules_lifecycle() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    // List -- empty
    let resp = client
        .get(&format!("{}/api/automation-rules", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);

    // Create
    let resp = client
        .post(&format!("{}/api/automation-rules", base))
        .json(&json!({
            "name": "Test rule",
            "rule_type": "auto_approve",
            "pattern": "read_file:src/**"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let rule: Value = resp.json().await.unwrap();
    let rule_id = rule["id"].as_i64().unwrap();

    // List -- should have 1
    let resp = client
        .get(&format!("{}/api/automation-rules", base))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Delete
    let resp = client
        .delete(&format!("{}/api/automation-rules/{}", base, rule_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List -- should be empty again
    let resp = client
        .get(&format!("{}/api/automation-rules", base))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}
