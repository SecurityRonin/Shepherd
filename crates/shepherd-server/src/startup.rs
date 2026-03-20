use crate::build_router;
use crate::state::AppState;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shepherd_core::adapters::AdapterRegistry;
use shepherd_core::cloud::CloudClient;
use shepherd_core::config;
use shepherd_core::config::types::ShepherdConfig;
use shepherd_core::coordination::LockManager;
use shepherd_core::db;
use shepherd_core::dispatch::TaskDispatcher;
use shepherd_core::events::ServerEvent;
use shepherd_core::iterm2::{auth::default_auth_path, Iterm2Manager};
use shepherd_core::pty::{sandbox::SandboxProfile, PtyManager, PtyOutput};
use shepherd_core::yolo::YoloEngine;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

/// Convert PTY output to a TerminalOutput server event.
/// Extracted from the spawned task for testability.
pub(crate) fn pty_output_to_event(output: &PtyOutput) -> ServerEvent {
    let data = String::from_utf8_lossy(&output.data).to_string();
    ServerEvent::TerminalOutput {
        task_id: output.task_id,
        data,
    }
}

/// Forward a PTY output chunk through the dispatcher for session monitoring.
/// Extracted from the spawned task for testability.
pub(crate) async fn forward_pty_to_dispatcher(dispatcher: &TaskDispatcher, output: &PtyOutput) {
    let text = String::from_utf8_lossy(&output.data);
    let _ = dispatcher.handle_pty_output(output.task_id, &text).await;
}

/// Lockfile written to `~/.shepherd/server.json` while the server is running.
/// Allows CLI tools and the Tauri front-end to discover the running server.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub pid: u32,
    pub port: u16,
    pub started_at: String,
}

impl ServerInfo {
    /// Returns the canonical path for the server lockfile.
    pub fn path() -> PathBuf {
        config::shepherd_dir().join("server.json")
    }

    /// Write this info to a specific path.
    pub fn write_to(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Read from a specific path.
    pub fn read_from(path: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Persist this info to the lockfile.
    pub fn write(&self) -> Result<()> {
        self.write_to(&Self::path())
    }

    /// Read the current lockfile, returning `None` if it doesn't exist or is
    /// malformed.
    pub fn read() -> Option<Self> {
        Self::read_from(&Self::path())
    }

    /// Remove the lockfile (best-effort).
    pub fn remove() {
        let _ = std::fs::remove_file(Self::path());
    }
}

/// Start the Shepherd HTTP server and return the bound address, shared state,
/// and a `JoinHandle` for the server task.
///
/// This function extracts all server setup logic so it can be called from both
/// `main.rs` (standalone binary) and the Tauri desktop app (embedded).
pub async fn start_server(
    cfg: ShepherdConfig,
) -> Result<(SocketAddr, Arc<AppState>, JoinHandle<()>)> {
    let port = cfg.port;
    let max_agents = cfg.max_agents;

    // ---- database ----
    let db_path = config::shepherd_dir().join("db.sqlite");
    std::fs::create_dir_all(config::shepherd_dir())?;
    let conn = db::open(&db_path).context("opening database")?;

    // ---- adapters ----
    // Install bundled default adapter configs (no-op if they already exist).
    let user_adapters_dir = config::shepherd_dir().join("adapters");
    shepherd_core::adapters::install_defaults(&user_adapters_dir).ok();

    let mut adapters = AdapterRegistry::new();
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let dev_adapters = std::path::Path::new(&manifest_dir).join("../../adapters");
        adapters.load_dir(&dev_adapters).ok();
    }
    let exe_adapters = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .map(|p| p.join("adapters"))
        .unwrap_or_default();
    adapters.load_dir(&exe_adapters).ok();
    adapters.load_dir(&user_adapters_dir).ok();
    tracing::info!("Loaded {} adapters", adapters.len());

    // ---- YOLO rules ----
    let yolo = YoloEngine::load(&config::shepherd_dir().join("rules.yaml"))?;

    // ---- sandbox / PTY ----
    let sandbox = {
        let mut profile = SandboxProfile {
            enabled: cfg.sandbox.enabled,
            block_network: cfg.sandbox.block_network,
            ..Default::default()
        };
        profile
            .blocked_paths
            .extend(cfg.sandbox.extra_blocked_paths.iter().cloned());
        profile
    };
    let pty = PtyManager::new(max_agents, sandbox);

    // ---- Arc-wrap shared components ----
    let adapters = Arc::new(adapters);
    let yolo = Arc::new(yolo);
    let pty = Arc::new(pty);

    // ---- event bus ----
    let (event_tx, _) = broadcast::channel(256);

    // ---- PTY output forwarding ----
    let mut pty_rx = pty.subscribe_output();
    let event_tx_clone = event_tx.clone();
    tokio::spawn(async move {
        while let Ok(output) = pty_rx.recv().await {
            let event = pty_output_to_event(&output);
            let _ = event_tx_clone.send(event);
        }
    });

    // ---- iTerm2 manager ----
    let iterm2 = Arc::new(Iterm2Manager::new(default_auth_path()));

    // ---- cloud client ----
    let cloud_client = if cfg.cloud.cloud_generation_enabled {
        tracing::info!("Cloud generation enabled");
        Some(CloudClient::new())
    } else {
        tracing::info!("Cloud generation disabled");
        None
    };

    // ---- shared state ----
    let db = Arc::new(Mutex::new(conn));
    let state = Arc::new(AppState {
        db: db.clone(),
        config: cfg,
        adapters: adapters.clone(),
        yolo: yolo.clone(),
        pty: pty.clone(),
        event_tx: event_tx.clone(),
        llm_provider: None,
        iterm2: Some(iterm2.clone()),
        cloud_client,
    });

    // Start iTerm2 session watcher
    if let Some(ref mgr) = state.iterm2 {
        mgr.clone().spawn(state.db.clone(), state.event_tx.clone());
    }

    // ---- TaskDispatcher polling loop ----
    let dispatcher = Arc::new(
        TaskDispatcher::new(
            db,
            adapters,
            pty.clone(),
            yolo,
            Arc::new(Mutex::new(LockManager::new())),
            event_tx.clone(),
        )
        .with_max_agents(max_agents),
    );

    // Poll for queued tasks every 2 seconds
    let dispatcher_poll = dispatcher.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = dispatcher_poll.poll_and_dispatch().await {
                tracing::error!("Dispatch loop error: {}", e);
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    // Forward PTY output through dispatcher for session monitoring
    let dispatcher_monitor = dispatcher.clone();
    let mut pty_monitor_rx = pty.subscribe_output();
    tokio::spawn(async move {
        while let Ok(output) = pty_monitor_rx.recv().await {
            forward_pty_to_dispatcher(&dispatcher_monitor, &output).await;
        }
    });

    // ---- background sync ----
    if let Some(ref client) = state.cloud_client {
        let sync_client = client.clone();
        tokio::spawn(shepherd_core::cloud::sync::background_sync(
            sync_client,
            shepherd_core::cloud::sync::SYNC_INTERVAL,
        ));
        tracing::info!("Background sync task spawned (interval=5m)");
    }

    // ---- HTTP server ----
    let app = build_router(state.clone());
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .with_context(|| format!("binding to 127.0.0.1:{port}"))?;
    let bound_addr = listener.local_addr()?;

    // Write server.json lockfile
    let info = ServerInfo {
        pid: std::process::id(),
        port: bound_addr.port(),
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    info.write()?;

    // Spawn the server task
    let state_shutdown = state.clone();
    let handle = tokio::spawn(async move {
        tracing::info!("Shepherd server listening on {}", bound_addr);
        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                tracing::info!("Shutting down — stopping all agents...");
                state_shutdown
                    .pty
                    .shutdown_all(std::time::Duration::from_secs(10))
                    .await;
            })
            .await
        {
            tracing::error!("Server error: {}", e);
        }
        ServerInfo::remove();
    });

    Ok((bound_addr, state, handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shepherd_core::yolo::rules::RuleSet;

    #[test]
    fn server_info_serde_roundtrip() {
        let info = ServerInfo {
            pid: 12345,
            port: 7532,
            started_at: "2026-03-19T10:00:00Z".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ServerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pid, 12345);
        assert_eq!(parsed.port, 7532);
        assert_eq!(parsed.started_at, "2026-03-19T10:00:00Z");
    }

    #[test]
    fn server_info_write_to_and_read_from() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sub/server.json");
        let info = ServerInfo {
            pid: 999,
            port: 8080,
            started_at: "2026-03-19T12:00:00Z".into(),
        };
        // Use the actual write_to method (creates parent dirs)
        info.write_to(&path).unwrap();
        // Use the actual read_from method
        let parsed = ServerInfo::read_from(&path).unwrap();
        assert_eq!(parsed.pid, 999);
        assert_eq!(parsed.port, 8080);
        assert_eq!(parsed.started_at, "2026-03-19T12:00:00Z");
    }

    #[test]
    fn server_info_write_to_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("server.json");
        let info1 = ServerInfo {
            pid: 1,
            port: 1000,
            started_at: "2026-01-01T00:00:00Z".into(),
        };
        info1.write_to(&path).unwrap();

        let info2 = ServerInfo {
            pid: 2,
            port: 2000,
            started_at: "2026-02-01T00:00:00Z".into(),
        };
        info2.write_to(&path).unwrap();

        let parsed = ServerInfo::read_from(&path).unwrap();
        assert_eq!(parsed.pid, 2);
        assert_eq!(parsed.port, 2000);
    }

    #[test]
    fn server_info_path_ends_with_server_json() {
        let path = ServerInfo::path();
        assert!(path.ends_with("server.json"));
        assert!(path.to_string_lossy().contains(".shepherd"));
    }

    #[test]
    fn server_info_deserializes_from_json_string() {
        let json = r#"{"pid":42,"port":3000,"started_at":"2026-01-01T00:00:00Z"}"#;
        let info: ServerInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.pid, 42);
        assert_eq!(info.port, 3000);
        assert_eq!(info.started_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn server_info_read_from_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.json");
        assert!(ServerInfo::read_from(&path).is_none());
    }

    #[test]
    fn server_info_read_from_malformed_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("server.json");
        std::fs::write(&path, "not json at all {{{}}}").unwrap();
        assert!(ServerInfo::read_from(&path).is_none());
    }

    #[test]
    fn server_info_read_from_partial_json_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("server.json");
        // Missing required field 'started_at'
        std::fs::write(&path, r#"{"pid":1,"port":80}"#).unwrap();
        assert!(ServerInfo::read_from(&path).is_none());
    }

    #[test]
    fn server_info_write_to_creates_nested_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("a/b/c/server.json");
        let info = ServerInfo {
            pid: 1,
            port: 3000,
            started_at: "2026-03-19T00:00:00Z".into(),
        };
        info.write_to(&path).unwrap();
        assert!(path.exists());
        let parsed = ServerInfo::read_from(&path).unwrap();
        assert_eq!(parsed.port, 3000);
    }

    #[test]
    fn server_info_remove_is_best_effort() {
        // Calling remove() when the file doesn't exist should not panic
        // We can't easily test this without touching the global path,
        // but we can verify it doesn't panic by calling it.
        // (This at least covers the remove() method body.)
        ServerInfo::remove();
    }

    #[test]
    fn server_info_write_and_read_global_path() {
        // Exercise write() and read() through the global path delegates.
        // These call write_to(path()) and read_from(path()) internally.
        let info = ServerInfo {
            pid: std::process::id(),
            port: 9999,
            started_at: "2026-03-19T00:00:00Z".into(),
        };
        info.write().unwrap();
        let parsed = ServerInfo::read().unwrap();
        assert_eq!(parsed.pid, std::process::id());
        assert_eq!(parsed.port, 9999);
        // Clean up
        ServerInfo::remove();
        assert!(ServerInfo::read().is_none());
    }

    #[test]
    fn pty_output_to_event_converts_correctly() {
        let output = PtyOutput {
            task_id: 42,
            data: b"hello world".to_vec(),
        };
        let event = super::pty_output_to_event(&output);
        match event {
            ServerEvent::TerminalOutput { task_id, data } => {
                assert_eq!(task_id, 42);
                assert_eq!(data, "hello world");
            }
            other => panic!("Expected TerminalOutput, got {:?}", other),
        }
    }

    #[test]
    fn pty_output_to_event_handles_invalid_utf8() {
        let output = PtyOutput {
            task_id: 7,
            data: vec![0xFF, 0xFE, 0x48, 0x69], // Invalid UTF-8 prefix + "Hi"
        };
        let event = super::pty_output_to_event(&output);
        match event {
            ServerEvent::TerminalOutput { task_id, data } => {
                assert_eq!(task_id, 7);
                // String::from_utf8_lossy replaces invalid bytes with U+FFFD
                assert!(data.contains("Hi"));
            }
            other => panic!("Expected TerminalOutput, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn forward_pty_to_dispatcher_handles_output() {
        use shepherd_core::coordination::LockManager;
        use shepherd_core::dispatch::TaskDispatcher;

        let (tx, _rx) = broadcast::channel(16);
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::default());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);

        let output = PtyOutput {
            task_id: 99,
            data: b"some agent output".to_vec(),
        };
        // Should not panic — no monitor for task 99, returns None internally
        super::forward_pty_to_dispatcher(&dispatcher, &output).await;
    }
}
