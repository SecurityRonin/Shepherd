use serde::{Deserialize, Serialize};

use super::{auth, CloudClient, CloudError};
use crate::logogen::{LogoGenInput, LogoStyle};

/// Request payload for the /api/generate/logo endpoint.
#[derive(Debug, Serialize)]
pub struct CloudLogoRequest {
    pub product_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_description: Option<String>,
    pub style: LogoStyle,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<String>,
    pub variants: u8,
}

impl From<&LogoGenInput> for CloudLogoRequest {
    fn from(input: &LogoGenInput) -> Self {
        Self {
            product_name: input.product_name.clone(),
            product_description: input.product_description.clone(),
            style: input.style.clone(),
            colors: input.colors.clone(),
            variants: input.variants,
        }
    }
}

/// Response from the /api/generate/logo endpoint.
#[derive(Debug, Deserialize)]
pub struct CloudLogoResponse {
    pub variants: Vec<CloudLogoVariant>,
    pub credits_remaining: u32,
}

/// A single logo variant from the cloud.
#[derive(Debug, Deserialize)]
pub struct CloudLogoVariant {
    pub index: u8,
    pub url: String,
}

/// Request payload for the /api/generate/name endpoint.
#[derive(Debug, Serialize)]
pub struct CloudNameRequest {
    pub description: String,
    pub vibes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// Response from the /api/generate/name endpoint.
#[derive(Debug, Deserialize)]
pub struct CloudNameResponse {
    pub candidates: Vec<CloudNameCandidate>,
    pub credits_remaining: u32,
}

/// A name candidate from the cloud.
#[derive(Debug, Deserialize)]
pub struct CloudNameCandidate {
    pub name: String,
    pub tagline: Option<String>,
    pub reasoning: String,
    pub domains: Vec<CloudDomainCheck>,
}

/// Domain availability check result.
#[derive(Debug, Deserialize)]
pub struct CloudDomainCheck {
    pub domain: String,
    pub available: bool,
}

/// Request payload for the /api/generate/northstar endpoint.
#[derive(Debug, Serialize)]
pub struct CloudNorthStarRequest {
    pub phase: String,
    pub context: serde_json::Value,
}

/// Response from the /api/generate/northstar endpoint.
#[derive(Debug, Deserialize)]
pub struct CloudNorthStarResponse {
    pub phase: String,
    pub result: serde_json::Value,
    pub credits_remaining: u32,
}

impl CloudClient {
    /// Generate logos via the cloud API.
    #[tracing::instrument(skip(self, input))]
    pub async fn generate_logo(
        &self,
        input: &LogoGenInput,
    ) -> Result<CloudLogoResponse, CloudError> {
        let jwt = auth::load_jwt().ok_or(CloudError::NotAuthenticated)?;
        let url = format!("{}/api/generate/logo", self.api_url());

        let request = CloudLogoRequest::from(input);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .json(&request)
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        if status == 401 {
            return Err(CloudError::AuthExpired);
        }
        if status == 402 {
            return Err(CloudError::InsufficientCredits {
                required: super::CREDIT_COST_LOGO,
                available: 0,
            });
        }
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CloudError::Api {
                status,
                message: body,
            });
        }

        resp.json()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))
    }

    /// Generate names via the cloud API.
    #[tracing::instrument(skip(self))]
    pub async fn generate_name(
        &self,
        description: &str,
        vibes: &[String],
    ) -> Result<CloudNameResponse, CloudError> {
        let jwt = auth::load_jwt().ok_or(CloudError::NotAuthenticated)?;
        let url = format!("{}/api/generate/name", self.api_url());

        let request = CloudNameRequest {
            description: description.to_string(),
            vibes: vibes.to_vec(),
            count: None,
        };

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .json(&request)
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        if status == 401 {
            return Err(CloudError::AuthExpired);
        }
        if status == 402 {
            return Err(CloudError::InsufficientCredits {
                required: super::CREDIT_COST_NAME,
                available: 0,
            });
        }
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CloudError::Api {
                status,
                message: body,
            });
        }

        resp.json()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))
    }

    /// Execute a North Star phase via the cloud API.
    #[tracing::instrument(skip(self, context))]
    pub async fn generate_northstar(
        &self,
        phase: &str,
        context: serde_json::Value,
    ) -> Result<CloudNorthStarResponse, CloudError> {
        let jwt = auth::load_jwt().ok_or(CloudError::NotAuthenticated)?;
        let url = format!("{}/api/generate/northstar", self.api_url());

        let request = CloudNorthStarRequest {
            phase: phase.to_string(),
            context,
        };

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .json(&request)
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        if status == 401 {
            return Err(CloudError::AuthExpired);
        }
        if status == 402 {
            return Err(CloudError::InsufficientCredits {
                required: super::CREDIT_COST_NORTHSTAR,
                available: 0,
            });
        }
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(CloudError::Api {
                status,
                message: body,
            });
        }

        resp.json()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_logo_request_from_input() {
        let input = LogoGenInput {
            product_name: "TestApp".to_string(),
            product_description: Some("A test app".to_string()),
            style: LogoStyle::Geometric,
            colors: vec!["#ff0000".to_string()],
            variants: 3,
        };

        let req = CloudLogoRequest::from(&input);
        assert_eq!(req.product_name, "TestApp");
        assert_eq!(req.product_description, Some("A test app".to_string()));
        assert_eq!(req.style, LogoStyle::Geometric);
        assert_eq!(req.colors.len(), 1);
        assert_eq!(req.variants, 3);
    }

    #[test]
    fn cloud_logo_request_serializes() {
        let req = CloudLogoRequest {
            product_name: "Foo".to_string(),
            product_description: None,
            style: LogoStyle::Minimal,
            colors: vec![],
            variants: 4,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"product_name\":\"Foo\""));
        // None fields and empty vecs should be skipped
        assert!(!json.contains("product_description"));
        assert!(!json.contains("colors"));
    }

    #[test]
    fn cloud_name_request_serializes() {
        let req = CloudNameRequest {
            description: "A project manager".to_string(),
            vibes: vec!["professional".to_string(), "modern".to_string()],
            count: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("project manager"));
        assert!(json.contains("professional"));
        assert!(!json.contains("count"));
    }

    #[test]
    fn cloud_northstar_request_serializes() {
        let req = CloudNorthStarRequest {
            phase: "brand_foundations".to_string(),
            context: serde_json::json!({ "name": "TestApp" }),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("brand_foundations"));
        assert!(json.contains("TestApp"));
    }

    #[test]
    fn cloud_logo_response_deserializes() {
        let json = r#"{
            "variants": [
                { "index": 0, "url": "https://example.com/logo0.png" },
                { "index": 1, "url": "https://example.com/logo1.png" }
            ],
            "credits_remaining": 48
        }"#;

        let resp: CloudLogoResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.variants.len(), 2);
        assert_eq!(resp.variants[0].index, 0);
        assert!(resp.variants[0].url.contains("logo0"));
        assert_eq!(resp.credits_remaining, 48);
    }

    #[test]
    fn cloud_name_response_deserializes() {
        let json = r#"{
            "candidates": [
                {
                    "name": "Acme",
                    "tagline": "Build better",
                    "reasoning": "Simple and memorable",
                    "domains": [
                        { "domain": "acme.dev", "available": true }
                    ]
                }
            ],
            "credits_remaining": 49
        }"#;

        let resp: CloudNameResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.candidates.len(), 1);
        assert_eq!(resp.candidates[0].name, "Acme");
        assert!(resp.candidates[0].domains[0].available);
        assert_eq!(resp.credits_remaining, 49);
    }
}
