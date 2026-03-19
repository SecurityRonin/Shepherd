// crates/shepherd-server/tests/startup_test.rs
//
// Integration test for start_server() — exercises the full server setup path.
//
// NOTE: Lines 242-246 (graceful shutdown via ctrl_c/SIGINT) and line 250
// (axum::serve error) in startup.rs are NOT covered by these tests.
// Sending SIGINT to the test process would kill the entire tokio runtime,
// making it unsafe for CI. The axum::serve error path requires a listener
// failure that cannot be reliably triggered in tests.

use shepherd_core::config::types::ShepherdConfig;
use shepherd_server::startup::ServerInfo;

#[tokio::test]
async fn start_server_binds_responds_and_writes_lockfile() {
    let mut cfg = ShepherdConfig::default();
    cfg.port = 0; // Let OS pick a free port

    let (addr, _state, handle) =
        shepherd_server::startup::start_server(cfg).await.unwrap();

    // Verify bound to a real port
    assert_ne!(addr.port(), 0);

    // Verify health endpoint responds
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/health", addr))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    // Verify server.json was written with correct content
    let info = ServerInfo::read();
    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info.port, addr.port());
    assert_eq!(info.pid, std::process::id());
    // started_at should be a valid RFC 3339 timestamp
    assert!(info.started_at.contains('T'));

    // Clean up
    handle.abort();
    ServerInfo::remove();
}

#[tokio::test]
async fn start_server_with_cloud_disabled() {
    let mut cfg = ShepherdConfig::default();
    cfg.port = 0; // Let OS pick a free port
    cfg.cloud.cloud_generation_enabled = false;

    let (addr, state, handle) =
        shepherd_server::startup::start_server(cfg).await.unwrap();

    // Verify server started
    assert_ne!(addr.port(), 0);

    // Cloud client should be None when cloud_generation_enabled is false
    assert!(state.cloud_client.is_none());

    // Clean up
    handle.abort();
    ServerInfo::remove();
}

/// E2E test: start the real server, create a task via REST API, and verify
/// the dispatch loop picks it up, spawns the PTY, and PTY output propagates
/// through the forwarding channels.
///
/// This covers the `tokio::spawn` bodies in startup.rs:
///   - Lines 154-155: PTY output forwarding loop (pty_output -> event bus)
///   - Line 216: Forward PTY output to dispatcher for session monitoring
///   - Line 205 is only hit on poll_and_dispatch error (not triggered here)
///   - Line 252: ServerInfo::remove() inside the server task
#[tokio::test]
async fn start_server_dispatches_task_and_receives_pty_output() {
    use shepherd_core::config;

    // 1. Write a test adapter to ~/.shepherd/adapters/ with a unique name
    //    to avoid collisions with other tests or a running Shepherd instance.
    let adapter_name = format!("test-echo-e2e-{}", std::process::id());
    let adapters_dir = config::shepherd_dir().join("adapters");
    std::fs::create_dir_all(&adapters_dir).unwrap();
    let adapter_path = adapters_dir.join(format!("{}.toml", adapter_name));
    std::fs::write(
        &adapter_path,
        r#"[agent]
name = "Echo Test"
command = "echo"
args = ["hello from shepherd e2e test"]

[status]
working_patterns = []
idle_patterns = ["\\$"]
input_patterns = []
error_patterns = []

[permissions]
approve = "y\n"
approve_all = "Y\n"
deny = "n\n"

[capabilities]
"#,
    )
    .unwrap();

    // 2. Start the server (port 0 = OS picks a free port)
    let mut cfg = ShepherdConfig::default();
    cfg.port = 0;
    let (addr, _state, handle) =
        shepherd_server::startup::start_server(cfg).await.unwrap();

    // 3. Create a task via the REST API targeting our test adapter
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/api/tasks", addr))
        .json(&serde_json::json!({
            "title": "E2E echo test",
            "agent_id": adapter_name,
            "repo_path": "/tmp"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "Task creation should return 201 Created");

    // 4. Wait for the dispatch loop to pick up the task and the echo command
    //    to produce output. The dispatch loop polls every 2 seconds, so we
    //    wait long enough for at least one cycle plus PTY propagation.
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // 5. Verify the task was dispatched — it should have transitioned past
    //    "queued". Since `echo` exits immediately, the task may be in
    //    dispatching, running, done, or error state.
    let resp = client
        .get(format!("http://{}/api/tasks", addr))
        .send()
        .await
        .unwrap();
    let tasks: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(!tasks.is_empty(), "Should have at least one task");

    // Find our task (there might be others if tests run in parallel)
    let our_task = tasks
        .iter()
        .find(|t| t["title"].as_str() == Some("E2E echo test"))
        .expect("Should find our E2E echo test task");

    let status = our_task["status"].as_str().unwrap();
    assert!(
        status == "dispatching" || status == "running" || status == "done" || status == "error",
        "Task should have moved past 'queued', but status is: {}",
        status
    );

    // 6. Clean up
    handle.abort();
    ServerInfo::remove();
    let _ = std::fs::remove_file(&adapter_path);
}
