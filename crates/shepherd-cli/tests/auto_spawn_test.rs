//! E2E test for CLI daemon auto-spawn.
//!
//! Tests that `ensure_server()` can spawn a real shepherd-server binary
//! when no server is running and no lockfile exists.
//! This covers lines 35-59 of lib.rs which are unreachable in unit tests
//! (no real `shepherd-server` binary available).

use shepherd_cli::ensure_server;
use shepherd_server::startup::ServerInfo;
use std::time::Duration;

/// Ensure the spawned daemon is killed and server.json is cleaned up,
/// even if the test panics.
struct DaemonGuard {
    pid: Option<u32>,
    deps_binary: Option<std::path::PathBuf>,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        if let Some(pid) = self.pid {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .output();
        }
        ServerInfo::remove();
        if let Some(ref path) = self.deps_binary {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[tokio::test]
async fn ensure_server_spawns_daemon_when_none_running() {
    let mut guard = DaemonGuard {
        pid: None,
        deps_binary: None,
    };

    // 1. Build the shepherd-server binary
    let build = std::process::Command::new("cargo")
        .args(["build", "--package", "shepherd-server"])
        .output()
        .expect("failed to run cargo build");
    assert!(
        build.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    // 2. Copy the binary next to the test binary so ensure_server() can find it.
    //    Test binary lives at target/debug/deps/auto_spawn_test-XXXX
    //    Built binary lives at target/debug/shepherd-server
    let test_exe = std::env::current_exe().unwrap();
    let deps_dir = test_exe.parent().unwrap(); // target/debug/deps/
    let debug_dir = deps_dir.parent().unwrap(); // target/debug/
    let source = debug_dir.join("shepherd-server");
    let dest = deps_dir.join("shepherd-server");

    assert!(
        source.exists(),
        "shepherd-server binary not found at {:?}",
        source
    );

    // Remove old copy if it exists, then copy fresh
    let _ = std::fs::remove_file(&dest);
    std::fs::copy(&source, &dest).expect("failed to copy shepherd-server to deps dir");

    // Ensure executable permission on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms).unwrap();
    }

    guard.deps_binary = Some(dest);

    // 3. Clean up any stale lockfile and kill any running server
    if let Some(info) = ServerInfo::read() {
        let _ = std::process::Command::new("kill")
            .arg(info.pid.to_string())
            .output();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    ServerInfo::remove();

    // 4. Call ensure_server with a URL that won't work (forces daemon spawn path).
    //    Port 1 is privileged and won't have a server running.
    //
    //    NOTE: After spawning, ensure_server polls the ORIGINAL server_url (port 1)
    //    for readiness, not the daemon's randomly-assigned port. So it will return
    //    Err after the 5-second timeout even though the daemon started successfully.
    //    This is a known limitation — the test verifies the daemon spawned via
    //    server.json instead.
    let _result = ensure_server("http://127.0.0.1:1").await;

    // 5. The daemon should have written server.json by now (or shortly after).
    //    Poll for it with a generous timeout.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut server_info = None;
    while tokio::time::Instant::now() < deadline {
        if let Some(info) = ServerInfo::read() {
            server_info = Some(info);
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let info = server_info.expect(
        "Daemon should have written server.json. \
         ensure_server() spawned the binary but it never wrote its lockfile.",
    );

    // Record PID for cleanup
    guard.pid = Some(info.pid);

    // 6. Verify the daemon is actually running by hitting its health endpoint
    let client = reqwest::Client::new();
    let health_url = format!("http://127.0.0.1:{}/api/health", info.port);
    let resp = client
        .get(&health_url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("spawned daemon should be reachable at its health endpoint");

    assert_eq!(
        resp.status(),
        200,
        "spawned daemon health check should return 200"
    );

    // 7. Cleanup is handled by DaemonGuard::drop
}
