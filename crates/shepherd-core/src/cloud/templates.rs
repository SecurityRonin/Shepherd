use super::CloudClient;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    Workflow,
    Pipeline,
    Pair,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentRole {
    pub role: String,
    pub agent_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub agents: Vec<AgentRole>,
    pub quality_gates: Vec<String>,
    pub is_premium: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplatesResponse {
    pub templates: Vec<AgentTemplate>,
}

impl CloudClient {
    pub async fn list_templates(
        &self,
        category: Option<&str>,
        include_premium: bool,
    ) -> Result<Vec<AgentTemplate>, super::CloudError> {
        let mut url = format!("{}/api/templates", self.api_url());
        let mut params = vec![];
        if let Some(cat) = category {
            params.push(format!("category={cat}"));
        }
        if !include_premium {
            params.push("include_premium=false".to_string());
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let resp = self
            .http
            .get(&url)
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

        let result: TemplatesResponse = resp
            .json()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;
        Ok(result.templates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_category_deserializes() {
        let json = "\"pipeline\"";
        let cat: TemplateCategory = serde_json::from_str(json).unwrap();
        assert_eq!(cat, TemplateCategory::Pipeline);
    }

    #[test]
    fn template_category_all_variants() {
        for (json, expected) in [
            ("\"workflow\"", TemplateCategory::Workflow),
            ("\"pipeline\"", TemplateCategory::Pipeline),
            ("\"pair\"", TemplateCategory::Pair),
        ] {
            let cat: TemplateCategory = serde_json::from_str(json).unwrap();
            assert_eq!(cat, expected);
        }
    }

    #[test]
    fn agent_template_deserializes() {
        let json = r#"{
            "id": "tdd-pipeline",
            "name": "TDD Pipeline",
            "description": "Three-agent TDD workflow",
            "category": "pipeline",
            "agents": [{"role": "planner", "agent_type": "claude-code", "config": {"focus": "test-first"}}],
            "quality_gates": ["lint", "test"],
            "is_premium": false
        }"#;
        let template: AgentTemplate = serde_json::from_str(json).unwrap();
        assert_eq!(template.id, "tdd-pipeline");
        assert_eq!(template.agents.len(), 1);
        assert!(!template.is_premium);
    }

    #[test]
    fn templates_response_deserializes() {
        let json = r#"{"templates":[
            {"id":"t1","name":"T1","description":"D1","category":"workflow",
             "agents":[{"role":"r","agent_type":"claude-code","config":{}}],
             "quality_gates":["test"],"is_premium":false},
            {"id":"t2","name":"T2","description":"D2","category":"pair",
             "agents":[{"role":"r","agent_type":"claude-code","config":{}}],
             "quality_gates":["lint"],"is_premium":true}
        ]}"#;
        let resp: TemplatesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.templates.len(), 2);
        assert!(resp.templates[1].is_premium);
    }

    #[test]
    fn agent_role_config_is_flexible() {
        let json =
            r#"{"role":"dev","agent_type":"claude-code","config":{"focus":"impl","max_turns":10}}"#;
        let role: AgentRole = serde_json::from_str(json).unwrap();
        assert_eq!(role.config["max_turns"], 10);
    }

    // ── httpmock-based async tests ────────────────────────────────────────

    fn templates_response_json() -> serde_json::Value {
        serde_json::json!({
            "templates": [
                {
                    "id": "tdd-pipeline",
                    "name": "TDD Pipeline",
                    "description": "Three-agent TDD workflow",
                    "category": "pipeline",
                    "agents": [{"role": "planner", "agent_type": "claude-code", "config": {"focus": "test-first"}}],
                    "quality_gates": ["lint", "test"],
                    "is_premium": false
                },
                {
                    "id": "pair-review",
                    "name": "Pair Review",
                    "description": "Two-agent pair programming",
                    "category": "pair",
                    "agents": [
                        {"role": "driver", "agent_type": "claude-code", "config": {}},
                        {"role": "reviewer", "agent_type": "claude-code", "config": {}}
                    ],
                    "quality_gates": ["lint"],
                    "is_premium": true
                }
            ]
        })
    }

    #[tokio::test]
    async fn list_templates_200_no_filters() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/templates");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(templates_response_json());
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let templates = client.list_templates(None, true).await.unwrap();
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].id, "tdd-pipeline");
        assert!(!templates[0].is_premium);
        assert_eq!(templates[1].id, "pair-review");
        assert!(templates[1].is_premium);
    }

    #[tokio::test]
    async fn list_templates_200_with_category_filter() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET)
                .path("/api/templates")
                .query_param("category", "pipeline")
                .query_param("include_premium", "false");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({
                    "templates": [{
                        "id": "tdd-pipeline",
                        "name": "TDD Pipeline",
                        "description": "Three-agent TDD workflow",
                        "category": "pipeline",
                        "agents": [{"role": "planner", "agent_type": "claude-code", "config": {}}],
                        "quality_gates": ["lint", "test"],
                        "is_premium": false
                    }]
                }));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let templates = client
            .list_templates(Some("pipeline"), false)
            .await
            .unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].category, TemplateCategory::Pipeline);
    }

    #[tokio::test]
    async fn list_templates_200_empty() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/templates");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({"templates": []}));
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let templates = client.list_templates(None, true).await.unwrap();
        assert!(templates.is_empty());
    }

    #[tokio::test]
    async fn list_templates_500_server_error() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/templates");
            then.status(500).body("Internal Server Error");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let result = client.list_templates(None, true).await;
        match result {
            Err(super::super::CloudError::Api { status, message }) => {
                assert_eq!(status, 500);
                assert!(message.contains("Internal Server Error"));
            }
            other => panic!("expected Api error with status 500, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn list_templates_404_not_found() {
        use httpmock::prelude::*;
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/templates");
            then.status(404).body("Not Found");
        });

        let client = super::super::CloudClient::with_test_jwt(&server.base_url(), "fake-jwt");
        let result = client.list_templates(None, true).await;
        match result {
            Err(super::super::CloudError::Api { status, .. }) => assert_eq!(status, 404),
            other => panic!("expected Api error with status 404, got: {:?}", other),
        }
    }
}
