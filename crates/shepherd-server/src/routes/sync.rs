use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use shepherd_core::cloud;
use shepherd_core::cloud::sync::SyncConfigPayload;
use std::sync::Arc;

use crate::state::AppState;

/// Response returned after a successful config push.
#[derive(Debug, Serialize)]
pub struct SyncPushResponse {
    pub success: bool,
}

/// Response returned after a successful config pull.
#[derive(Debug, Serialize)]
pub struct SyncPullResponse {
    pub config: Option<cloud::sync::SyncConfigEntry>,
}

/// Response returned after triggering an immediate sync.
#[derive(Debug, Serialize)]
pub struct SyncNowResponse {
    pub success: bool,
}

/// Resolve the machine identifier from config, falling back to the OS hostname.
fn resolve_machine_id(state: &AppState) -> String {
    state
        .config
        .cloud
        .sync_machine_id
        .clone()
        .unwrap_or_else(|| {
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "unknown".to_string())
        })
}

/// Map a [`cloud::CloudError`] to an axum error tuple.
fn cloud_error_to_response(e: cloud::CloudError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, msg) = match &e {
        cloud::CloudError::NotAuthenticated | cloud::CloudError::AuthExpired => {
            (StatusCode::UNAUTHORIZED, e.to_string())
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

/// Require the cloud client, returning 503 if absent.
fn require_cloud_client(
    state: &AppState,
) -> Result<&cloud::CloudClient, (StatusCode, Json<serde_json::Value>)> {
    state.cloud_client.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Cloud features not available"
            })),
        )
    })
}

/// POST /api/sync/push -- push current config to cloud.
#[tracing::instrument(skip(state))]
pub async fn sync_push(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SyncPushResponse>, (StatusCode, Json<serde_json::Value>)> {
    let cloud = require_cloud_client(&state)?;

    let machine_id = resolve_machine_id(&state);
    let config_value = serde_json::to_value(&state.config).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to serialize config: {e}") })),
        )
    })?;

    let payload = SyncConfigPayload {
        machine_id,
        config: config_value,
        version: 1,
    };

    cloud
        .push_config(&payload)
        .await
        .map_err(cloud_error_to_response)?;

    Ok(Json(SyncPushResponse { success: true }))
}

/// POST /api/sync/pull -- pull config from cloud.
#[tracing::instrument(skip(state))]
pub async fn sync_pull(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SyncPullResponse>, (StatusCode, Json<serde_json::Value>)> {
    let cloud = require_cloud_client(&state)?;

    let machine_id = resolve_machine_id(&state);
    let entry = cloud
        .pull_config(&machine_id)
        .await
        .map_err(cloud_error_to_response)?;

    Ok(Json(SyncPullResponse { config: entry }))
}

/// POST /api/sync/now -- trigger immediate sync (balance refresh).
#[tracing::instrument(skip(state))]
pub async fn sync_now_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SyncNowResponse>, (StatusCode, Json<serde_json::Value>)> {
    let cloud = require_cloud_client(&state)?;

    cloud::sync::sync_now(cloud)
        .await
        .map_err(cloud_error_to_response)?;

    Ok(Json(SyncNowResponse { success: true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Response serialization tests ──────────────────────────────────

    #[test]
    fn sync_push_response_serializes() {
        let resp = SyncPushResponse { success: true };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["success"].as_bool().unwrap());
        let obj = json.as_object().unwrap();
        assert_eq!(obj.len(), 1);
    }

    #[test]
    fn sync_push_response_serializes_false() {
        let resp = SyncPushResponse { success: false };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(!json["success"].as_bool().unwrap());
    }

    #[test]
    fn sync_pull_response_serializes_with_config() {
        let entry = cloud::sync::SyncConfigEntry {
            machine_id: "mbp-2024".to_string(),
            config: json!({"yolo_mode": false}),
            version: 3,
            updated_at: "2026-03-20T12:00:00Z".to_string(),
        };
        let resp = SyncPullResponse {
            config: Some(entry),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(!json["config"].is_null());
        assert_eq!(json["config"]["machine_id"], "mbp-2024");
        assert_eq!(json["config"]["version"], 3);
    }

    #[test]
    fn sync_pull_response_serializes_without_config() {
        let resp = SyncPullResponse { config: None };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["config"].is_null());
    }

    #[test]
    fn sync_now_response_serializes() {
        let resp = SyncNowResponse { success: true };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["success"].as_bool().unwrap());
        let obj = json.as_object().unwrap();
        assert_eq!(obj.len(), 1);
    }

    // ── 503 when cloud not configured ─────────────────────────────────

    #[test]
    fn require_cloud_client_returns_503_when_none() {
        use rusqlite::Connection;
        use shepherd_core::adapters::AdapterRegistry;
        use shepherd_core::config::types::ShepherdConfig;
        use shepherd_core::pty::sandbox::SandboxProfile;
        use shepherd_core::pty::PtyManager;
        use shepherd_core::yolo::rules::RuleSet;
        use shepherd_core::yolo::YoloEngine;
        use tokio::sync::broadcast;
        use tokio::sync::Mutex;

        let (tx, _rx) = broadcast::channel(16);
        let db = Connection::open_in_memory().unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            config: ShepherdConfig::default(),
            adapters: Arc::new(AdapterRegistry::default()),
            yolo: Arc::new(YoloEngine::new(RuleSet {
                deny: vec![],
                allow: vec![],
            })),
            pty: Arc::new(PtyManager::new(10, SandboxProfile::disabled())),
            event_tx: tx,
            llm_provider: None,
            iterm2: None,
            cloud_client: None,
        };

        let result = require_cloud_client(&state);
        assert!(result.is_err());
        let (status, body) = result.unwrap_err();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let error_json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&body.0).unwrap()).unwrap();
        assert_eq!(error_json["error"], "Cloud features not available");
    }

    #[test]
    fn require_cloud_client_returns_ok_when_some() {
        use rusqlite::Connection;
        use shepherd_core::adapters::AdapterRegistry;
        use shepherd_core::config::types::ShepherdConfig;
        use shepherd_core::pty::sandbox::SandboxProfile;
        use shepherd_core::pty::PtyManager;
        use shepherd_core::yolo::rules::RuleSet;
        use shepherd_core::yolo::YoloEngine;
        use tokio::sync::broadcast;
        use tokio::sync::Mutex;

        let (tx, _rx) = broadcast::channel(16);
        let db = Connection::open_in_memory().unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            config: ShepherdConfig::default(),
            adapters: Arc::new(AdapterRegistry::default()),
            yolo: Arc::new(YoloEngine::new(RuleSet {
                deny: vec![],
                allow: vec![],
            })),
            pty: Arc::new(PtyManager::new(10, SandboxProfile::disabled())),
            event_tx: tx,
            llm_provider: None,
            iterm2: None,
            cloud_client: Some(cloud::CloudClient::new()),
        };

        let result = require_cloud_client(&state);
        assert!(result.is_ok());
    }

    // ── Error mapping tests ───────────────────────────────────────────

    #[test]
    fn cloud_error_not_authenticated_maps_to_401() {
        let (status, body) = cloud_error_to_response(cloud::CloudError::NotAuthenticated);
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&body.0).unwrap()).unwrap();
        assert!(json["error"].as_str().unwrap().contains("Not signed in"));
    }

    #[test]
    fn cloud_error_auth_expired_maps_to_401() {
        let (status, body) = cloud_error_to_response(cloud::CloudError::AuthExpired);
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&body.0).unwrap()).unwrap();
        assert!(json["error"].as_str().unwrap().contains("expired"));
    }

    #[test]
    fn cloud_error_network_maps_to_500() {
        let (status, _body) =
            cloud_error_to_response(cloud::CloudError::Network("timeout".to_string()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn cloud_error_api_maps_to_500() {
        let (status, _body) = cloud_error_to_response(cloud::CloudError::Api {
            status: 502,
            message: "Bad Gateway".to_string(),
        });
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    // ── Machine ID resolution tests ───────────────────────────────────

    #[test]
    fn resolve_machine_id_uses_config_when_set() {
        use rusqlite::Connection;
        use shepherd_core::adapters::AdapterRegistry;
        use shepherd_core::config::types::ShepherdConfig;
        use shepherd_core::pty::sandbox::SandboxProfile;
        use shepherd_core::pty::PtyManager;
        use shepherd_core::yolo::rules::RuleSet;
        use shepherd_core::yolo::YoloEngine;
        use tokio::sync::broadcast;
        use tokio::sync::Mutex;

        let (tx, _rx) = broadcast::channel(16);
        let db = Connection::open_in_memory().unwrap();
        let mut config = ShepherdConfig::default();
        config.cloud.sync_machine_id = Some("my-custom-id".to_string());
        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            config,
            adapters: Arc::new(AdapterRegistry::default()),
            yolo: Arc::new(YoloEngine::new(RuleSet {
                deny: vec![],
                allow: vec![],
            })),
            pty: Arc::new(PtyManager::new(10, SandboxProfile::disabled())),
            event_tx: tx,
            llm_provider: None,
            iterm2: None,
            cloud_client: None,
        };

        assert_eq!(resolve_machine_id(&state), "my-custom-id");
    }

    #[test]
    fn resolve_machine_id_falls_back_to_hostname() {
        use rusqlite::Connection;
        use shepherd_core::adapters::AdapterRegistry;
        use shepherd_core::config::types::ShepherdConfig;
        use shepherd_core::pty::sandbox::SandboxProfile;
        use shepherd_core::pty::PtyManager;
        use shepherd_core::yolo::rules::RuleSet;
        use shepherd_core::yolo::YoloEngine;
        use tokio::sync::broadcast;
        use tokio::sync::Mutex;

        let (tx, _rx) = broadcast::channel(16);
        let db = Connection::open_in_memory().unwrap();
        let state = AppState {
            db: Arc::new(Mutex::new(db)),
            config: ShepherdConfig::default(), // sync_machine_id is None
            adapters: Arc::new(AdapterRegistry::default()),
            yolo: Arc::new(YoloEngine::new(RuleSet {
                deny: vec![],
                allow: vec![],
            })),
            pty: Arc::new(PtyManager::new(10, SandboxProfile::disabled())),
            event_tx: tx,
            llm_provider: None,
            iterm2: None,
            cloud_client: None,
        };

        let machine_id = resolve_machine_id(&state);
        // Should be the actual hostname, not empty or "unknown" on a real system
        assert!(!machine_id.is_empty());
    }
}
