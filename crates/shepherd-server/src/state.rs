use rusqlite::Connection;
use shepherd_core::adapters::AdapterRegistry;
use shepherd_core::config::types::ShepherdConfig;
use shepherd_core::events::ServerEvent;
use shepherd_core::pty::PtyManager;
use shepherd_core::yolo::YoloEngine;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub config: ShepherdConfig,
    pub adapters: AdapterRegistry,
    pub yolo: YoloEngine,
    pub pty: PtyManager,
    pub event_tx: broadcast::Sender<ServerEvent>,
    pub llm_provider: Option<Box<dyn shepherd_core::llm::LlmProvider>>,
}
