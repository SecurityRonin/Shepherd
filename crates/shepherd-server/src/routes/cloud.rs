use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use shepherd_core::cloud;
use std::sync::Arc;

use crate::state::AppState;

/// Cloud status response — tells the frontend what's available.
#[derive(Debug, Serialize)]
pub struct CloudStatusResponse {
    pub cloud_available: bool,
    pub authenticated: bool,
    pub plan: Option<String>,
    pub credits_balance: Option<u32>,
    pub cloud_generation_enabled: bool,
}

/// GET /api/cloud/status — cloud availability and auth state.
#[tracing::instrument(skip(state))]
pub async fn cloud_status(State(state): State<Arc<AppState>>) -> Json<CloudStatusResponse> {
    let cloud_available = state.cloud_client.is_some();
    let cloud_generation_enabled = state.config.cloud.cloud_generation_enabled;

    let (authenticated, plan, credits_balance) = if cloud_available {
        let is_authed = cloud::auth::is_authenticated();
        if is_authed {
            let profile = cloud::auth::load_cached_profile();
            match profile {
                Some(p) => (true, Some(p.plan.to_string()), Some(p.credits_balance)),
                None => (true, None, None),
            }
        } else {
            (false, None, None)
        }
    } else {
        (false, None, None)
    };

    Json(CloudStatusResponse {
        cloud_available,
        authenticated,
        plan,
        credits_balance,
        cloud_generation_enabled,
    })
}

/// Credit balance response.
#[derive(Debug, Serialize)]
pub struct CreditBalanceResponse {
    pub plan: String,
    pub credits_balance: u32,
    pub subscription_url: String,
    pub topup_url: String,
}

/// GET /api/cloud/balance — refresh and return credit balance.
#[tracing::instrument(skip(state))]
pub async fn cloud_balance(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CreditBalanceResponse>, (StatusCode, Json<serde_json::Value>)> {
    let cloud = state.cloud_client.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Cloud features not available"
            })),
        )
    })?;

    let balance = cloud.refresh_balance().await.map_err(|e| {
        let (status, msg) = match &e {
            cloud::CloudError::NotAuthenticated | cloud::CloudError::AuthExpired => {
                (StatusCode::UNAUTHORIZED, e.to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (status, Json(serde_json::json!({ "error": msg })))
    })?;

    let profile = cloud::auth::load_cached_profile();
    let plan = profile
        .map(|p| p.plan.to_string())
        .unwrap_or_else(|| "free".to_string());

    Ok(Json(CreditBalanceResponse {
        plan,
        credits_balance: balance,
        subscription_url: cloud.subscription_url(),
        topup_url: cloud.topup_url(),
    }))
}

/// Feature cost info for the frontend.
#[derive(Debug, Serialize)]
pub struct FeatureCostResponse {
    pub features: Vec<FeatureCost>,
}

/// Cost information for a single feature.
#[derive(Debug, Serialize)]
pub struct FeatureCost {
    pub name: String,
    pub credits: u32,
}

/// GET /api/cloud/costs — list credit costs per feature.
pub async fn cloud_costs() -> Json<FeatureCostResponse> {
    Json(FeatureCostResponse {
        features: vec![
            FeatureCost {
                name: "logo".into(),
                credits: cloud::CREDIT_COST_LOGO,
            },
            FeatureCost {
                name: "name".into(),
                credits: cloud::CREDIT_COST_NAME,
            },
            FeatureCost {
                name: "northstar".into(),
                credits: cloud::CREDIT_COST_NORTHSTAR,
            },
            FeatureCost {
                name: "scrape".into(),
                credits: cloud::CREDIT_COST_SCRAPE,
            },
            FeatureCost {
                name: "crawl".into(),
                credits: cloud::CREDIT_COST_CRAWL,
            },
            FeatureCost {
                name: "vision".into(),
                credits: cloud::CREDIT_COST_VISION,
            },
            FeatureCost {
                name: "search".into(),
                credits: cloud::CREDIT_COST_SEARCH,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_status_response_serialize() {
        let resp = CloudStatusResponse {
            cloud_available: true,
            authenticated: false,
            plan: None,
            credits_balance: None,
            cloud_generation_enabled: true,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["cloud_available"].as_bool().unwrap());
        assert!(!json["authenticated"].as_bool().unwrap());
        assert!(json["plan"].is_null());
        assert!(json["credits_balance"].is_null());
        assert!(json["cloud_generation_enabled"].as_bool().unwrap());
    }

    #[test]
    fn cloud_status_response_with_auth() {
        let resp = CloudStatusResponse {
            cloud_available: true,
            authenticated: true,
            plan: Some("pro".to_string()),
            credits_balance: Some(42),
            cloud_generation_enabled: true,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["authenticated"].as_bool().unwrap());
        assert_eq!(json["plan"], "pro");
        assert_eq!(json["credits_balance"], 42);
    }

    #[test]
    fn credit_balance_response_serialize() {
        let resp = CreditBalanceResponse {
            plan: "pro".to_string(),
            credits_balance: 85,
            subscription_url: "https://api.shepherd.codes/api/credits/purchase".to_string(),
            topup_url: "https://api.shepherd.codes/api/credits/purchase".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["plan"], "pro");
        assert_eq!(json["credits_balance"], 85);
        assert!(json["subscription_url"]
            .as_str()
            .unwrap()
            .contains("/credits/purchase"));
    }

    #[test]
    fn feature_cost_response_has_all_features() {
        let resp = futures::executor::block_on(cloud_costs());
        assert_eq!(resp.features.len(), 7);
        let names: Vec<&str> = resp.features.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"logo"));
        assert!(names.contains(&"name"));
        assert!(names.contains(&"northstar"));
        assert!(names.contains(&"scrape"));
        assert!(names.contains(&"crawl"));
        assert!(names.contains(&"vision"));
        assert!(names.contains(&"search"));
    }

    #[test]
    fn feature_costs_match_constants() {
        let resp = futures::executor::block_on(cloud_costs());
        for f in &resp.features {
            let expected = match f.name.as_str() {
                "logo" => cloud::CREDIT_COST_LOGO,
                "name" => cloud::CREDIT_COST_NAME,
                "northstar" => cloud::CREDIT_COST_NORTHSTAR,
                "scrape" => cloud::CREDIT_COST_SCRAPE,
                "crawl" => cloud::CREDIT_COST_CRAWL,
                "vision" => cloud::CREDIT_COST_VISION,
                "search" => cloud::CREDIT_COST_SEARCH,
                _ => panic!("unexpected feature: {}", f.name),
            };
            assert_eq!(f.credits, expected, "cost mismatch for {}", f.name);
        }
    }

    #[test]
    fn cloud_status_no_cloud() {
        let resp = CloudStatusResponse {
            cloud_available: false,
            authenticated: false,
            plan: None,
            credits_balance: None,
            cloud_generation_enabled: false,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(!json["cloud_available"].as_bool().unwrap());
        assert!(!json["cloud_generation_enabled"].as_bool().unwrap());
    }

    #[test]
    fn feature_cost_serialize() {
        let cost = FeatureCost {
            name: "logo".to_string(),
            credits: 5,
        };
        let json = serde_json::to_value(&cost).unwrap();
        assert_eq!(json["name"], "logo");
        assert_eq!(json["credits"], 5);
    }

    #[test]
    fn feature_cost_response_serialize() {
        let resp = FeatureCostResponse {
            features: vec![
                FeatureCost {
                    name: "logo".to_string(),
                    credits: 5,
                },
                FeatureCost {
                    name: "name".to_string(),
                    credits: 3,
                },
            ],
        };
        let json = serde_json::to_value(&resp).unwrap();
        let features = json["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0]["name"], "logo");
        assert_eq!(features[1]["credits"], 3);
    }

    #[test]
    fn credit_balance_response_all_fields() {
        let resp = CreditBalanceResponse {
            plan: "free".to_string(),
            credits_balance: 0,
            subscription_url: "https://example.com/subscribe".to_string(),
            topup_url: "https://example.com/topup".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["plan"], "free");
        assert_eq!(json["credits_balance"], 0);
        assert!(json["subscription_url"]
            .as_str()
            .unwrap()
            .starts_with("https://"));
        assert!(json["topup_url"].as_str().unwrap().starts_with("https://"));
    }
}
