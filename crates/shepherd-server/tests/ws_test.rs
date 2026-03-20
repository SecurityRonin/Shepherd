// crates/shepherd-server/tests/ws_test.rs
//
// WebSocket integration tests — verifies the full event pipeline:
// client connects, sends events, receives server broadcasts.

use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

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

    tokio::time::sleep(Duration::from_millis(150)).await;
    (url, handle)
}

fn ws_url(http_url: &str) -> String {
    http_url.replace("http://", "ws://") + "/ws"
}

#[tokio::test]
async fn ws_connect_and_receive_task_created_event() {
    let (url, _handle) = start_test_server().await;

    // Connect a WebSocket client
    let (mut ws, _resp) = connect_async(ws_url(&url))
        .await
        .expect("WebSocket connection failed");

    // Send Subscribe event
    let subscribe = json!({"type": "subscribe", "data": null});
    ws.send(Message::Text(subscribe.to_string().into()))
        .await
        .unwrap();

    // Create a task via REST — the WS handler also supports task creation,
    // but let's use REST to test the broadcast pipeline end-to-end.
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/api/tasks"))
        .json(&json!({
            "title": "WS test task",
            "agent_id": "claude-code"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = resp["id"].as_i64().unwrap();

    // Listen for the task_created event on the WebSocket
    let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("Timed out waiting for WS message")
        .expect("WS stream ended")
        .expect("WS message error");

    let event: Value = match &msg {
        Message::Text(text) => serde_json::from_str(text.as_ref()).unwrap(),
        other => panic!("Expected text message, got {:?}", other),
    };

    assert_eq!(event["type"], "task_created");
    assert_eq!(event["data"]["id"], task_id);
    assert_eq!(event["data"]["title"], "WS test task");
    assert_eq!(event["data"]["agent_id"], "claude-code");
    assert_eq!(event["data"]["status"], "queued");
}

#[tokio::test]
async fn ws_task_create_via_websocket() {
    let (url, _handle) = start_test_server().await;

    // Connect a WebSocket client
    let (mut ws, _resp) = connect_async(ws_url(&url))
        .await
        .expect("WebSocket connection failed");

    // Create a task via WebSocket ClientEvent
    let create_event = json!({
        "type": "task_create",
        "data": {
            "title": "WS-created task",
            "agent_id": "claude-code",
            "repo_path": null,
            "isolation_mode": null,
            "prompt": null
        }
    });
    ws.send(Message::Text(create_event.to_string().into()))
        .await
        .unwrap();

    // Should receive TaskCreated event back on the same connection
    let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("Timed out waiting for WS message")
        .expect("WS stream ended")
        .expect("WS message error");

    let event: Value = match &msg {
        Message::Text(text) => serde_json::from_str(text.as_ref()).unwrap(),
        other => panic!("Expected text message, got {:?}", other),
    };

    assert_eq!(event["type"], "task_created");
    assert_eq!(event["data"]["title"], "WS-created task");
    assert_eq!(event["data"]["agent_id"], "claude-code");
    assert_eq!(event["data"]["status"], "queued");

    // Verify via REST that the task persisted in the database
    let client = reqwest::Client::new();
    let tasks: Vec<Value> = client
        .get(format!("{url}/api/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "WS-created task");
}

#[tokio::test]
async fn ws_delete_task_broadcasts_event() {
    let (url, _handle) = start_test_server().await;

    // Connect WebSocket first so we receive broadcast events
    let (mut ws, _resp) = connect_async(ws_url(&url))
        .await
        .expect("WebSocket connection failed");

    // Create a task via REST
    let client = reqwest::Client::new();
    let resp: Value = client
        .post(format!("{url}/api/tasks"))
        .json(&json!({
            "title": "Delete me",
            "agent_id": "claude-code"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = resp["id"].as_i64().unwrap();

    // Consume the task_created event
    let _created = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("Timed out")
        .expect("Stream ended")
        .expect("Error");

    // Delete via REST
    client
        .delete(format!("{url}/api/tasks/{task_id}"))
        .send()
        .await
        .unwrap();

    // Should receive task_deleted event
    let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("Timed out waiting for delete event")
        .expect("WS stream ended")
        .expect("WS message error");

    let event: Value = match &msg {
        Message::Text(text) => serde_json::from_str(text.as_ref()).unwrap(),
        other => panic!("Expected text message, got {:?}", other),
    };

    assert_eq!(event["type"], "task_deleted");
    assert_eq!(event["data"]["id"], task_id);
}

#[tokio::test]
async fn ws_multiple_clients_receive_broadcasts() {
    let (url, _handle) = start_test_server().await;

    // Connect two WebSocket clients
    let (mut ws1, _) = connect_async(ws_url(&url))
        .await
        .expect("WS1 connection failed");
    let (mut ws2, _) = connect_async(ws_url(&url))
        .await
        .expect("WS2 connection failed");

    // Create a task via REST
    let client = reqwest::Client::new();
    client
        .post(format!("{url}/api/tasks"))
        .json(&json!({
            "title": "Broadcast test",
            "agent_id": "claude-code"
        }))
        .send()
        .await
        .unwrap();

    // Both clients should receive the task_created event
    let msg1 = tokio::time::timeout(Duration::from_secs(5), ws1.next())
        .await
        .expect("WS1 timed out")
        .expect("WS1 stream ended")
        .expect("WS1 error");
    let msg2 = tokio::time::timeout(Duration::from_secs(5), ws2.next())
        .await
        .expect("WS2 timed out")
        .expect("WS2 stream ended")
        .expect("WS2 error");

    let event1: Value = match &msg1 {
        Message::Text(text) => serde_json::from_str(text.as_ref()).unwrap(),
        other => panic!("WS1: expected text, got {:?}", other),
    };
    let event2: Value = match &msg2 {
        Message::Text(text) => serde_json::from_str(text.as_ref()).unwrap(),
        other => panic!("WS2: expected text, got {:?}", other),
    };

    assert_eq!(event1["type"], "task_created");
    assert_eq!(event2["type"], "task_created");
    assert_eq!(event1["data"]["title"], "Broadcast test");
    assert_eq!(event2["data"]["title"], "Broadcast test");
}

#[tokio::test]
async fn ws_close_connection_gracefully() {
    let (url, _handle) = start_test_server().await;

    let (mut ws, _) = connect_async(ws_url(&url))
        .await
        .expect("WS connection failed");

    // Close the WebSocket
    ws.close(None).await.unwrap();

    // Server should handle this gracefully — verify by making a REST call
    let client = reqwest::Client::new();
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
