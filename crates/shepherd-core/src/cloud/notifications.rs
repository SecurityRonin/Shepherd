use serde::{Deserialize, Serialize};
use super::CloudClient;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEvent {
    AgentFinished,
    AgentFailed,
    GateFailed,
    BudgetExceeded,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationPayload {
    #[serde(rename = "type")]
    pub event_type: NotificationEvent,
    pub agent_id: String,
    pub machine_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_usd: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationPreferences {
    pub slack_webhook_url: Option<String>,
    pub email_enabled: bool,
    pub events: Vec<NotificationEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationPreferencesResponse {
    pub preferences: NotificationPreferences,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendNotificationResponse {
    pub sent: bool,
    pub reason: Option<String>,
}

impl CloudClient {
    pub async fn get_notification_preferences(&self) -> Result<NotificationPreferences, super::CloudError> {
        let jwt = self.get_jwt()?;
        let url = format!("{}/api/notifications/preferences", self.api_url());
        let resp = self.http.get(&url)
            .bearer_auth(&jwt)
            .send()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(super::CloudError::Api { status, message: body });
        }

        let result: NotificationPreferencesResponse = resp.json().await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;
        Ok(result.preferences)
    }

    pub async fn send_notification(&self, payload: &NotificationPayload) -> Result<bool, super::CloudError> {
        let jwt = self.get_jwt()?;
        let url = format!("{}/api/notifications/send", self.api_url());
        let resp = self.http.post(&url)
            .bearer_auth(&jwt)
            .json(payload)
            .send()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(super::CloudError::Api { status, message: body });
        }

        let result: SendNotificationResponse = resp.json().await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;
        Ok(result.sent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_event_serializes() {
        let json = serde_json::to_string(&NotificationEvent::AgentFinished).unwrap();
        assert_eq!(json, "\"agent_finished\"");
    }

    #[test]
    fn notification_event_all_variants() {
        let events = vec![
            NotificationEvent::AgentFinished,
            NotificationEvent::AgentFailed,
            NotificationEvent::GateFailed,
            NotificationEvent::BudgetExceeded,
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let parsed: NotificationEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event, parsed);
        }
    }

    #[test]
    fn notification_payload_serializes() {
        let payload = NotificationPayload {
            event_type: NotificationEvent::AgentFinished,
            agent_id: "claude-code".to_string(),
            machine_id: "mbp-2024".to_string(),
            task_summary: Some("Built login page".to_string()),
            gate_name: None,
            error: None,
            cost_usd: None,
            budget_usd: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"type\":\"agent_finished\""));
        assert!(json.contains("claude-code"));
        assert!(!json.contains("gate_name"));
    }

    #[test]
    fn notification_payload_budget_exceeded() {
        let payload = NotificationPayload {
            event_type: NotificationEvent::BudgetExceeded,
            agent_id: "claude-code".to_string(),
            machine_id: "mbp-2024".to_string(),
            task_summary: None,
            gate_name: None,
            error: None,
            cost_usd: Some(5.50),
            budget_usd: Some(5.00),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("5.5"));
        assert!(json.contains("5.0"));
    }

    #[test]
    fn preferences_response_deserializes() {
        let json = r#"{"preferences":{"slack_webhook_url":"https://hooks.slack.com/T/B/x","email_enabled":true,"events":["agent_finished","gate_failed"]}}"#;
        let resp: NotificationPreferencesResponse = serde_json::from_str(json).unwrap();
        assert!(resp.preferences.email_enabled);
        assert_eq!(resp.preferences.events.len(), 2);
    }

    #[test]
    fn preferences_response_empty_events() {
        let json = r#"{"preferences":{"slack_webhook_url":null,"email_enabled":false,"events":[]}}"#;
        let resp: NotificationPreferencesResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.preferences.email_enabled);
        assert!(resp.preferences.events.is_empty());
    }

    #[test]
    fn send_response_sent() {
        let json = r#"{"sent":true,"reason":null}"#;
        let resp: SendNotificationResponse = serde_json::from_str(json).unwrap();
        assert!(resp.sent);
    }

    #[test]
    fn send_response_not_sent() {
        let json = r#"{"sent":false,"reason":"event_not_enabled"}"#;
        let resp: SendNotificationResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.sent);
        assert_eq!(resp.reason, Some("event_not_enabled".to_string()));
    }

    // ── httpmock-based async tests ────────────────────────────────────────

    fn make_test_payload() -> NotificationPayload {
        NotificationPayload {
            event_type: NotificationEvent::AgentFinished,
            agent_id: "claude-code".to_string(),
            machine_id: "mbp-2024".to_string(),
            task_summary: Some("Built login page".to_string()),
            gate_name: None,
            error: None,
            cost_usd: None,
            budget_usd: None,
        }
    }

    #[tokio::test]
    async fn get_notification_preferences_200_ok() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/notifications/preferences");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "preferences": {
                        "slack_webhook_url": "https://hooks.slack.com/T/B/x",
                        "email_enabled": true,
                        "events": ["agent_finished", "gate_failed"]
                    }
                }));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let prefs = client.get_notification_preferences().await.unwrap();
        assert!(prefs.email_enabled);
        assert_eq!(prefs.events.len(), 2);
        assert_eq!(prefs.events[0], NotificationEvent::AgentFinished);
        assert_eq!(prefs.events[1], NotificationEvent::GateFailed);
        assert_eq!(prefs.slack_webhook_url, Some("https://hooks.slack.com/T/B/x".to_string()));
    }

    #[tokio::test]
    async fn get_notification_preferences_401_auth_expired() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/notifications/preferences");
            then.status(401).body("Unauthorized");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "bad-jwt");
        let result = client.get_notification_preferences().await;
        match result {
            Err(super::super::CloudError::Api { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected Api error with status 401, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_notification_preferences_500_server_error() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/notifications/preferences");
            then.status(500).body("Internal Server Error");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let result = client.get_notification_preferences().await;
        match result {
            Err(super::super::CloudError::Api { status, message }) => {
                assert_eq!(status, 500);
                assert!(message.contains("Internal Server Error"));
            }
            other => panic!("expected Api error with status 500, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn send_notification_200_sent_true() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/notifications/send");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "sent": true,
                    "reason": null
                }));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let payload = make_test_payload();
        let sent = client.send_notification(&payload).await.unwrap();
        assert!(sent);
    }

    #[tokio::test]
    async fn send_notification_200_sent_false() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/notifications/send");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "sent": false,
                    "reason": "event_not_enabled"
                }));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let payload = make_test_payload();
        let sent = client.send_notification(&payload).await.unwrap();
        assert!(!sent);
    }

    #[tokio::test]
    async fn send_notification_401_auth_expired() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/notifications/send");
            then.status(401).body("Unauthorized");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "bad-jwt");
        let payload = make_test_payload();
        let result = client.send_notification(&payload).await;
        match result {
            Err(super::super::CloudError::Api { status, .. }) => assert_eq!(status, 401),
            other => panic!("expected Api error with status 401, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn send_notification_500_server_error() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/notifications/send");
            then.status(500).body("boom");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let payload = make_test_payload();
        let result = client.send_notification(&payload).await;
        match result {
            Err(super::super::CloudError::Api { status, message }) => {
                assert_eq!(status, 500);
                assert_eq!(message, "boom");
            }
            other => panic!("expected Api error with status 500, got: {:?}", other),
        }
    }
}
