use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use shepherd_core::cloud;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct TemplateQuery {
    pub category: Option<String>,
    #[serde(default = "default_true")]
    pub include_premium: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct TemplatesListResponse {
    pub templates: Vec<cloud::templates::AgentTemplate>,
}

/// GET /api/templates — list available agent templates.
#[tracing::instrument(skip(state))]
pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TemplateQuery>,
) -> Result<Json<TemplatesListResponse>, (StatusCode, Json<serde_json::Value>)> {
    let cloud = state.cloud_client.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Cloud features not available"
            })),
        )
    })?;

    let templates = cloud
        .list_templates(params.category.as_deref(), params.include_premium)
        .await
        .map_err(|e| {
            let (status, msg) = match &e {
                cloud::CloudError::NotAuthenticated | cloud::CloudError::AuthExpired => {
                    (StatusCode::UNAUTHORIZED, e.to_string())
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            };
            (status, Json(serde_json::json!({ "error": msg })))
        })?;

    Ok(Json(TemplatesListResponse { templates }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_list_response_serialize_empty() {
        let resp = TemplatesListResponse { templates: vec![] };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["templates"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn templates_list_response_serialize_with_items() {
        let template = cloud::templates::AgentTemplate {
            id: "tdd-pipeline".to_string(),
            name: "TDD Pipeline".to_string(),
            description: "Three-agent TDD workflow".to_string(),
            category: cloud::templates::TemplateCategory::Pipeline,
            agents: vec![cloud::templates::AgentRole {
                role: "planner".to_string(),
                agent_type: "claude-code".to_string(),
                config: serde_json::json!({"focus": "test-first"}),
            }],
            quality_gates: vec!["lint".to_string(), "test".to_string()],
            is_premium: false,
        };

        let resp = TemplatesListResponse {
            templates: vec![template],
        };
        let json = serde_json::to_value(&resp).unwrap();
        let templates = json["templates"].as_array().unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0]["id"], "tdd-pipeline");
        assert_eq!(templates[0]["category"], "pipeline");
        assert!(!templates[0]["is_premium"].as_bool().unwrap());
        assert_eq!(templates[0]["agents"][0]["role"], "planner");
    }

    #[test]
    fn template_query_defaults() {
        let query: TemplateQuery = serde_json::from_str("{}").unwrap();
        assert!(query.category.is_none());
        assert!(query.include_premium);
    }

    #[test]
    fn template_query_with_category() {
        let query: TemplateQuery =
            serde_json::from_str(r#"{"category":"pipeline","include_premium":false}"#).unwrap();
        assert_eq!(query.category.as_deref(), Some("pipeline"));
        assert!(!query.include_premium);
    }

    #[test]
    fn templates_response_serializes_premium_flag() {
        let premium = cloud::templates::AgentTemplate {
            id: "pro-workflow".to_string(),
            name: "Pro Workflow".to_string(),
            description: "Premium workflow".to_string(),
            category: cloud::templates::TemplateCategory::Workflow,
            agents: vec![],
            quality_gates: vec![],
            is_premium: true,
        };
        let free = cloud::templates::AgentTemplate {
            id: "free-pair".to_string(),
            name: "Free Pair".to_string(),
            description: "Free pairing".to_string(),
            category: cloud::templates::TemplateCategory::Pair,
            agents: vec![],
            quality_gates: vec![],
            is_premium: false,
        };

        let resp = TemplatesListResponse {
            templates: vec![premium, free],
        };
        let json = serde_json::to_value(&resp).unwrap();
        let templates = json["templates"].as_array().unwrap();
        assert!(templates[0]["is_premium"].as_bool().unwrap());
        assert!(!templates[1]["is_premium"].as_bool().unwrap());
    }

    #[test]
    fn templates_response_serializes_quality_gates() {
        let template = cloud::templates::AgentTemplate {
            id: "gated".to_string(),
            name: "Gated".to_string(),
            description: "Has gates".to_string(),
            category: cloud::templates::TemplateCategory::Pipeline,
            agents: vec![],
            quality_gates: vec![
                "lint".to_string(),
                "test".to_string(),
                "typecheck".to_string(),
            ],
            is_premium: false,
        };
        let resp = TemplatesListResponse {
            templates: vec![template],
        };
        let json = serde_json::to_value(&resp).unwrap();
        let gates = json["templates"][0]["quality_gates"].as_array().unwrap();
        assert_eq!(gates.len(), 3);
        assert_eq!(gates[0], "lint");
        assert_eq!(gates[1], "test");
        assert_eq!(gates[2], "typecheck");
    }
}
