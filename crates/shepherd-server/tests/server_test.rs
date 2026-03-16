// crates/shepherd-server/tests/server_test.rs
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let port = portpicker::pick_unused_port().expect("No free port");
    let url = format!("http://127.0.0.1:{port}");

    let handle = tokio::spawn(async move {
        let cfg = shepherd_core::config::types::ShepherdConfig {
            port,
            ..Default::default()
        };
        let conn = shepherd_core::db::open_memory().unwrap();
        let adapters = shepherd_core::adapters::AdapterRegistry::new();
        let yolo = shepherd_core::yolo::YoloEngine::new(
            shepherd_core::yolo::rules::RuleSet { deny: vec![], allow: vec![] },
        );
        let pty = shepherd_core::pty::PtyManager::new(cfg.max_agents, shepherd_core::pty::sandbox::SandboxProfile::disabled());
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
        });

        let app = shepherd_server::build_router(state);

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    (url, handle)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp: Value = client
        .get(format!("{url}/api/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["status"], "ok");
}

#[tokio::test]
async fn test_create_and_list_tasks() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();

    let resp = client
        .post(format!("{url}/api/tasks"))
        .json(&json!({
            "title": "Test task",
            "agent_id": "claude-code"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["title"], "Test task");
    assert_eq!(body["status"], "queued");

    let tasks: Vec<Value> = client
        .get(format!("{url}/api/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Test task");
}

#[tokio::test]
async fn test_delete_task() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();

    let resp: Value = client
        .post(format!("{url}/api/tasks"))
        .json(&json!({ "title": "To delete", "agent_id": "claude-code" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = resp["id"].as_i64().unwrap();

    let del: Value = client
        .delete(format!("{}/api/tasks/{}", url, id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(del["deleted"], id);

    let tasks: Vec<Value> = client
        .get(format!("{url}/api/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(tasks.is_empty());
}
