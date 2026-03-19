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

#[tokio::test]
async fn test_cloud_status_endpoint() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp: Value = client
        .get(format!("{url}/api/cloud/status"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // With cloud_client: None, cloud is not available
    assert_eq!(resp["cloud_available"], false);
    assert_eq!(resp["authenticated"], false);
    assert!(resp["plan"].is_null());
    assert!(resp["credits_balance"].is_null());
    // cloud_generation_enabled reflects the config default (true), not cloud availability
    assert_eq!(resp["cloud_generation_enabled"], true);
}

#[tokio::test]
async fn test_cloud_costs_endpoint() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp: Value = client
        .get(format!("{url}/api/cloud/costs"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let features = resp["features"].as_array().unwrap();
    assert_eq!(features.len(), 7);
    // Verify all expected feature names are present
    let names: Vec<&str> = features
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"logo"));
    assert!(names.contains(&"name"));
    assert!(names.contains(&"northstar"));
    assert!(names.contains(&"scrape"));
    assert!(names.contains(&"crawl"));
    assert!(names.contains(&"vision"));
    assert!(names.contains(&"search"));
    // All costs should be positive integers
    for feature in features {
        assert!(feature["credits"].as_u64().unwrap() > 0);
    }
}

#[tokio::test]
async fn test_northstar_phases_endpoint() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp: Value = client
        .get(format!("{url}/api/northstar/phases"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["total"], 13);
    let phases = resp["phases"].as_array().unwrap();
    assert_eq!(phases.len(), 13);
    // All phases should have sequential IDs starting at 1
    for (i, phase) in phases.iter().enumerate() {
        assert_eq!(phase["id"].as_u64().unwrap(), (i + 1) as u64);
        assert!(!phase["name"].as_str().unwrap().is_empty());
        assert!(!phase["description"].as_str().unwrap().is_empty());
        assert!(phase["document_count"].as_u64().unwrap() > 0);
    }
}

#[tokio::test]
async fn test_cloud_balance_unavailable() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp = client
        .get(format!("{url}/api/cloud/balance"))
        .send()
        .await
        .unwrap();
    // With cloud_client: None, should return 503
    assert_eq!(resp.status(), 503);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("not available"));
}

#[tokio::test]
async fn test_logogen_no_provider_returns_503() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp = client
        .post(format!("{url}/api/logogen"))
        .json(&json!({
            "product_name": "Test",
            "style": "minimal"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("No generation provider"));
}

#[tokio::test]
async fn test_namegen_no_provider_returns_503() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp = client
        .post(format!("{url}/api/namegen"))
        .json(&json!({
            "description": "A test product"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("No generation provider"));
}

#[tokio::test]
async fn test_northstar_execute_invalid_phase() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp = client
        .post(format!("{url}/api/northstar/phase"))
        .json(&json!({
            "product_name": "Test",
            "product_description": "A test",
            "phase_id": 99
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Unknown phase_id"));
}

#[tokio::test]
async fn test_northstar_execute_no_provider() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp = client
        .post(format!("{url}/api/northstar/phase"))
        .json(&json!({
            "product_name": "Test",
            "product_description": "A test",
            "phase_id": 1
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("No generation provider"));
}

#[tokio::test]
async fn test_gates_task_not_found() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp = client
        .post(format!("{url}/api/tasks/99999/gates"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_pr_task_not_found() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp = client
        .post(format!("{url}/api/tasks/99999/pr"))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_delete_nonexistent_task() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    // SQLite DELETE succeeds even when no row matches, so the handler returns 200
    let resp = client
        .delete(format!("{url}/api/tasks/99999"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["deleted"], 99999);
}

#[tokio::test]
async fn test_logogen_export_invalid_base64() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp = client
        .post(format!("{url}/api/logogen/export"))
        .json(&json!({
            "image_base64": "not-valid-base64",
            "product_name": "Test"
        }))
        .send()
        .await
        .unwrap();
    // Invalid base64 triggers the export error path
    assert_eq!(resp.status(), 500);
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("Icon export failed"));
}
