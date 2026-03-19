use rusqlite::Connection;
use shepherd_core::adapters::AdapterRegistry;
use shepherd_core::cloud::CloudClient;
use shepherd_core::config::types::ShepherdConfig;
use shepherd_core::events::ServerEvent;
use shepherd_core::iterm2::Iterm2Manager;
use shepherd_core::pty::PtyManager;
use shepherd_core::yolo::YoloEngine;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub config: ShepherdConfig,
    pub adapters: Arc<AdapterRegistry>,
    pub yolo: Arc<YoloEngine>,
    pub pty: Arc<PtyManager>,
    pub event_tx: broadcast::Sender<ServerEvent>,
    pub llm_provider: Option<Box<dyn shepherd_core::llm::LlmProvider>>,
    pub iterm2: Option<Arc<Iterm2Manager>>,
    pub cloud_client: Option<CloudClient>,
}

impl AppState {
    /// Whether cloud generation is available (client configured + feature enabled).
    pub fn cloud_generation_available(&self) -> bool {
        self.cloud_client.is_some() && self.config.cloud.cloud_generation_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shepherd_core::adapters::AdapterRegistry;
    use shepherd_core::config::types::ShepherdConfig;
    use shepherd_core::pty::sandbox::SandboxProfile;
    use shepherd_core::yolo::rules::RuleSet;
    use std::sync::Arc;

    /// Helper to build a minimal AppState for testing.
    fn make_state(cloud_client: Option<CloudClient>, cloud_generation_enabled: bool) -> AppState {
        let (tx, _rx) = broadcast::channel(16);
        let db = Connection::open_in_memory().unwrap();
        let mut config = ShepherdConfig::default();
        config.cloud.cloud_generation_enabled = cloud_generation_enabled;

        AppState {
            db: Arc::new(Mutex::new(db)),
            config,
            adapters: Arc::new(AdapterRegistry::default()),
            yolo: Arc::new(YoloEngine::new(RuleSet { deny: vec![], allow: vec![] })),
            pty: Arc::new(PtyManager::new(10, SandboxProfile::disabled())),
            event_tx: tx,
            llm_provider: None,
            iterm2: None,
            cloud_client,
        }
    }

    #[test]
    fn cloud_generation_available_no_client() {
        let state = make_state(None, true);
        assert!(!state.cloud_generation_available());
    }

    #[test]
    fn cloud_generation_available_disabled() {
        // Even without a real client, if cloud_client is None it's false regardless
        let state = make_state(None, false);
        assert!(!state.cloud_generation_available());
    }
}
