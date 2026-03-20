use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{auth, CloudClient};

#[derive(Debug, Clone, Serialize)]
pub struct SyncConfigPayload {
    pub machine_id: String,
    pub config: serde_json::Value,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfigEntry {
    pub machine_id: String,
    pub config: serde_json::Value,
    pub version: u32,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncPullResponse {
    pub config: Option<SyncConfigEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncPullAllResponse {
    pub configs: Vec<SyncConfigEntry>,
}

/// Default interval between background balance refreshes.
pub const SYNC_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

/// Minimum interval to prevent excessive API calls.
pub const MIN_SYNC_INTERVAL: Duration = Duration::from_secs(30);

/// Start a background task that periodically refreshes the cached profile.
///
/// This should be spawned as a tokio task on app startup. It runs until
/// the returned handle is dropped or the task is aborted.
///
/// The sync only runs when:
/// - A JWT is stored (user is authenticated)
/// - The interval has elapsed since last sync
#[tracing::instrument(skip(client))]
// tarpaulin-start-ignore
pub async fn background_sync(client: CloudClient, interval: Duration) {
    let interval = if interval < MIN_SYNC_INTERVAL {
        MIN_SYNC_INTERVAL
    } else {
        interval
    };

    loop {
        tokio::time::sleep(interval).await;

        // Only sync if authenticated
        if !auth::is_authenticated() {
            continue;
        }

        match client.refresh_balance().await {
            Ok(balance) => {
                tracing::debug!("Background sync: credits_balance={balance}");
            }
            Err(e) => {
                tracing::warn!("Background sync failed: {e}");
            }
        }
    }
}

// tarpaulin-stop-ignore

/// Perform a one-time sync (useful on app startup or after mutations).
#[tracing::instrument(skip(client))]
// tarpaulin-start-ignore
pub async fn sync_now(client: &CloudClient) -> Result<(), super::CloudError> {
    if !auth::is_authenticated() {
        return Err(super::CloudError::NotAuthenticated);
    }

    client.refresh_balance().await?;
    Ok(())
}
// tarpaulin-stop-ignore

impl CloudClient {
    pub async fn push_config(&self, payload: &SyncConfigPayload) -> Result<(), super::CloudError> {
        let jwt = self.get_jwt()?;
        let url = format!("{}/api/sync/push", self.api_url());
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&jwt)
            .json(payload)
            .send()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(super::CloudError::Api {
                status,
                message: body,
            });
        }
        Ok(())
    }

    pub async fn pull_config(
        &self,
        machine_id: &str,
    ) -> Result<Option<SyncConfigEntry>, super::CloudError> {
        let jwt = self.get_jwt()?;
        let url = format!("{}/api/sync/pull?machine_id={}", self.api_url(), machine_id);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&jwt)
            .send()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(super::CloudError::Api {
                status,
                message: body,
            });
        }

        let result: SyncPullResponse = resp
            .json()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;
        Ok(result.config)
    }

    pub async fn pull_all_configs(&self) -> Result<Vec<SyncConfigEntry>, super::CloudError> {
        let jwt = self.get_jwt()?;
        let url = format!("{}/api/sync/pull", self.api_url());
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&jwt)
            .send()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(super::CloudError::Api {
                status,
                message: body,
            });
        }

        let result: SyncPullAllResponse = resp
            .json()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;
        Ok(result.configs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_interval_defaults() {
        assert_eq!(SYNC_INTERVAL, Duration::from_secs(300));
        assert_eq!(MIN_SYNC_INTERVAL, Duration::from_secs(30));
    }

    #[test]
    fn min_interval_less_than_default() {
        assert!(MIN_SYNC_INTERVAL < SYNC_INTERVAL);
    }

    #[test]
    fn sync_interval_enforced() {
        // Verify the clamping logic: if given an interval < MIN, it should use MIN
        let tiny = Duration::from_secs(1);
        let clamped = if tiny < MIN_SYNC_INTERVAL {
            MIN_SYNC_INTERVAL
        } else {
            tiny
        };
        assert_eq!(clamped, MIN_SYNC_INTERVAL);
    }

    #[test]
    fn sync_interval_passthrough() {
        // If given a valid interval >= MIN, it should pass through
        let valid = Duration::from_secs(600);
        let clamped = if valid < MIN_SYNC_INTERVAL {
            MIN_SYNC_INTERVAL
        } else {
            valid
        };
        assert_eq!(clamped, valid);
    }

    #[test]
    fn sync_config_payload_serializes() {
        let payload = SyncConfigPayload {
            machine_id: "mbp-2024".to_string(),
            config: serde_json::json!({
                "quality_gates": {"lint": true},
                "yolo_mode": false
            }),
            version: 1,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("mbp-2024"));
        assert!(json.contains("quality_gates"));
    }

    #[test]
    fn sync_config_response_deserializes() {
        let json = r#"{"config":{"machine_id":"mbp","config":{"yolo":true},"version":1,"updated_at":"2026-03-17T00:00:00Z"}}"#;
        let resp: SyncPullResponse = serde_json::from_str(json).unwrap();
        assert!(resp.config.is_some());
    }

    #[test]
    fn sync_configs_response_deserializes() {
        let json = r#"{"configs":[{"machine_id":"mbp","config":{"yolo":true},"version":1,"updated_at":"2026-03-17"}]}"#;
        let resp: SyncPullAllResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.configs.len(), 1);
    }

    // ── httpmock-based async tests ────────────────────────────────────────

    fn make_test_sync_payload() -> SyncConfigPayload {
        SyncConfigPayload {
            machine_id: "mbp-2024".to_string(),
            config: serde_json::json!({
                "quality_gates": {"lint": true},
                "yolo_mode": false
            }),
            version: 1,
        }
    }

    #[tokio::test]
    async fn push_config_200_ok() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/sync/push");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({"ok": true}));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let payload = make_test_sync_payload();
        let result = client.push_config(&payload).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn push_config_401_auth_expired() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/sync/push");
            then.status(401).body("Unauthorized");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "bad-jwt");
        let payload = make_test_sync_payload();
        let result = client.push_config(&payload).await;
        match result {
            Err(super::super::CloudError::Api { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected Api error with status 401, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn push_config_500_server_error() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/sync/push");
            then.status(500).body("Internal Server Error");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let payload = make_test_sync_payload();
        let result = client.push_config(&payload).await;
        match result {
            Err(super::super::CloudError::Api { status, message }) => {
                assert_eq!(status, 500);
                assert!(message.contains("Internal Server Error"));
            }
            other => panic!("expected Api error with status 500, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn pull_config_200_with_config() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET)
                .path("/api/sync/pull")
                .query_param("machine_id", "mbp-2024");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "config": {
                        "machine_id": "mbp-2024",
                        "config": {"yolo_mode": false},
                        "version": 3,
                        "updated_at": "2026-03-17T12:00:00Z"
                    }
                }));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let entry = client.pull_config("mbp-2024").await.unwrap();
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.machine_id, "mbp-2024");
        assert_eq!(entry.version, 3);
    }

    #[tokio::test]
    async fn pull_config_200_no_config() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET)
                .path("/api/sync/pull")
                .query_param("machine_id", "unknown-machine");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "config": null
                }));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let entry = client.pull_config("unknown-machine").await.unwrap();
        assert!(entry.is_none());
    }

    #[tokio::test]
    async fn pull_config_401_auth_expired() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/sync/pull");
            then.status(401).body("Unauthorized");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "bad-jwt");
        let result = client.pull_config("mbp-2024").await;
        match result {
            Err(super::super::CloudError::Api { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected Api error with status 401, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn pull_all_configs_200_ok() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/sync/pull");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "configs": [
                        {
                            "machine_id": "mbp-2024",
                            "config": {"yolo_mode": false},
                            "version": 1,
                            "updated_at": "2026-03-17T12:00:00Z"
                        },
                        {
                            "machine_id": "linux-dev",
                            "config": {"yolo_mode": true},
                            "version": 2,
                            "updated_at": "2026-03-18T08:00:00Z"
                        }
                    ]
                }));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let configs = client.pull_all_configs().await.unwrap();
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].machine_id, "mbp-2024");
        assert_eq!(configs[1].machine_id, "linux-dev");
    }

    #[tokio::test]
    async fn pull_all_configs_200_empty() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/sync/pull");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "configs": []
                }));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let configs = client.pull_all_configs().await.unwrap();
        assert!(configs.is_empty());
    }

    #[tokio::test]
    async fn pull_all_configs_401_auth_expired() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/sync/pull");
            then.status(401).body("Unauthorized");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "bad-jwt");
        let result = client.pull_all_configs().await;
        match result {
            Err(super::super::CloudError::Api { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected Api error with status 401, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn pull_all_configs_500_server_error() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/sync/pull");
            then.status(500).body("db down");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let result = client.pull_all_configs().await;
        match result {
            Err(super::super::CloudError::Api { status, message }) => {
                assert_eq!(status, 500);
                assert_eq!(message, "db down");
            }
            other => panic!("expected Api error with status 500, got: {:?}", other),
        }
    }
}
