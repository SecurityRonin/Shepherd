// crates/shepherd-server/tests/startup_test.rs
//
// Integration test for start_server() — exercises the full server setup path.

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
