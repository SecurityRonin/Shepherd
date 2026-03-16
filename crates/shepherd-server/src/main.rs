use shepherd_core::{adapters::AdapterRegistry, config, db, iterm2::{auth::default_auth_path, Iterm2Manager}, pty::{PtyManager, sandbox::SandboxProfile}, yolo::YoloEngine};
use shepherd_server::state::AppState;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("shepherd=info".parse()?),
        )
        .init();

    let cfg = config::load_config(None)?;
    let port = cfg.port;
    let max_agents = cfg.max_agents;

    let db_path = config::shepherd_dir().join("db.sqlite");
    std::fs::create_dir_all(config::shepherd_dir())?;
    let conn = db::open(&db_path)?;

    let mut adapters = AdapterRegistry::new();
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let dev_adapters = std::path::Path::new(&manifest_dir).join("../../adapters");
        adapters.load_dir(&dev_adapters).ok();
    }
    let exe_adapters = std::env::current_exe()?
        .parent()
        .unwrap()
        .join("adapters");
    adapters.load_dir(&exe_adapters).ok();
    adapters
        .load_dir(&config::shepherd_dir().join("adapters"))
        .ok();
    tracing::info!("Loaded {} adapters", adapters.len());

    let yolo = YoloEngine::load(&config::shepherd_dir().join("rules.yaml"))?;

    let sandbox = {
        let mut profile = SandboxProfile {
            enabled: cfg.sandbox.enabled,
            block_network: cfg.sandbox.block_network,
            ..Default::default()
        };
        profile.blocked_paths.extend(cfg.sandbox.extra_blocked_paths.iter().cloned());
        profile
    };
    let pty = PtyManager::new(max_agents, sandbox);

    let (event_tx, _) = broadcast::channel(256);

    let mut pty_rx = pty.subscribe_output();
    let event_tx_clone = event_tx.clone();
    tokio::spawn(async move {
        while let Ok(output) = pty_rx.recv().await {
            let data = String::from_utf8_lossy(&output.data).to_string();
            let _ =
                event_tx_clone.send(shepherd_core::events::ServerEvent::TerminalOutput {
                    task_id: output.task_id,
                    data,
                });
        }
    });

    let iterm2 = Arc::new(Iterm2Manager::new(default_auth_path()));

    let state = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        config: cfg,
        adapters,
        yolo,
        pty,
        event_tx,
        llm_provider: None,
        iterm2: Some(iterm2.clone()),
    });

    if let Some(ref mgr) = state.iterm2 {
        mgr.clone().spawn(state.db.clone(), state.event_tx.clone());
    }

    let app = shepherd_server::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    tracing::info!("Shepherd server listening on http://127.0.0.1:{port}");

    let state_shutdown = state.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("Shutting down — stopping all agents...");
            state_shutdown
                .pty
                .shutdown_all(std::time::Duration::from_secs(10))
                .await;
        })
        .await?;
    Ok(())
}
