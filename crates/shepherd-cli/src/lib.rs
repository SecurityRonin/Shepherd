//! Shepherd CLI library — shared logic for the CLI binary.

use anyhow::{Context, Result};
use reqwest::Client;

/// Ensure a Shepherd server is running. Returns the base URL to use.
///
/// Checks (in order):
/// 1. The configured URL's health endpoint
/// 2. A server.json lockfile for an existing daemon
/// 3. Spawns a new server daemon and polls for readiness
pub async fn ensure_server(server_url: &str) -> Result<String> {
    let client = Client::new();

    // 1. Try the configured URL directly
    if let Ok(resp) = client.get(&format!("{server_url}/api/health")).send().await {
        if resp.status().is_success() {
            return Ok(server_url.to_string());
        }
    }

    // 2. Check server.json for an existing daemon
    if let Some(info) = shepherd_server::startup::ServerInfo::read() {
        let url = format!("http://127.0.0.1:{}", info.port);
        if let Ok(resp) = client.get(&format!("{url}/api/health")).send().await {
            if resp.status().is_success() {
                return Ok(url);
            }
        }
        // Stale server.json — clean it up
        shepherd_server::startup::ServerInfo::remove();
    }

    // 3. Spawn server daemon
    eprintln!("Starting shepherd server daemon...");
    let exe = std::env::current_exe()?;
    let server_binary = exe
        .parent()
        .map(|p| p.join("shepherd-server"))
        .unwrap_or_else(|| "shepherd-server".into());

    std::process::Command::new(&server_binary)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to start shepherd-server daemon. Is it installed?")?;

    // 4. Wait for server to become ready (poll up to 5 seconds)
    for _ in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Ok(resp) = client.get(&format!("{server_url}/api/health")).send().await {
            if resp.status().is_success() {
                eprintln!("Server started successfully.");
                return Ok(server_url.to_string());
            }
        }
    }

    anyhow::bail!("Server failed to start within 5 seconds")
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests share global server.json state, so they must run sequentially.
    // Use `cargo test -p shepherd-cli --lib -- --test-threads=1` for reliability,
    // or each test carefully manages the lockfile to avoid race conditions.

    #[tokio::test]
    async fn ensure_server_connects_to_running_server() {
        // Clean up any stale lockfile first so step 2 doesn't interfere
        shepherd_server::startup::ServerInfo::remove();

        // Start a real server on a random port
        let mut cfg = shepherd_core::config::types::ShepherdConfig::default();
        cfg.port = 0;
        let (addr, _state, handle) = shepherd_server::startup::start_server(cfg).await.unwrap();
        let url = format!("http://127.0.0.1:{}", addr.port());

        // ensure_server should find it immediately via health check (step 1)
        let result = ensure_server(&url).await;
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        assert_eq!(result.unwrap(), url);

        handle.abort();
        shepherd_server::startup::ServerInfo::remove();
    }

    #[tokio::test]
    async fn ensure_server_finds_server_via_lockfile() {
        // Clean up any stale lockfile from other tests
        shepherd_server::startup::ServerInfo::remove();

        // Start a real server on a random port
        let mut cfg = shepherd_core::config::types::ShepherdConfig::default();
        cfg.port = 0;
        let (addr, _state, handle) = shepherd_server::startup::start_server(cfg).await.unwrap();
        let port = addr.port();

        // Explicitly write our own server.json with the correct port,
        // overriding whatever start_server wrote (which may have been
        // clobbered by a concurrent test).
        let info = shepherd_server::startup::ServerInfo {
            pid: std::process::id(),
            port,
            started_at: String::from("test"),
        };
        info.write().unwrap();

        // Use a WRONG URL so the direct health check (step 1) fails,
        // forcing ensure_server to fall through to step 2 (lockfile)
        let result = ensure_server("http://127.0.0.1:1").await;
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
        let found_url = result.unwrap();
        assert!(
            found_url.contains(&port.to_string()),
            "expected URL to contain port {}, got: {}",
            port,
            found_url
        );

        handle.abort();
        shepherd_server::startup::ServerInfo::remove();
    }

    #[tokio::test]
    async fn ensure_server_fails_when_no_server_available() {
        // Clean up any existing server.json
        shepherd_server::startup::ServerInfo::remove();

        // Use a URL that definitely won't work
        let result = ensure_server("http://127.0.0.1:1").await;
        // This will try to spawn shepherd-server binary, which likely doesn't exist
        // in the test environment. Either spawn fails or the 5-second timeout fires.
        assert!(result.is_err(), "expected Err, got: {:?}", result);
    }
}
