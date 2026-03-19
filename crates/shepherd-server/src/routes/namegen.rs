use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use shepherd_core::cloud::CloudError;
use shepherd_core::namegen::{self, DomainCheck, NameCandidate, NameGenInput, ValidationStatus};
use std::sync::Arc;

use crate::state::AppState;

/// Request body for name generation.
#[derive(Debug, Deserialize)]
pub struct NameGenRequest {
    pub description: String,
    #[serde(default)]
    pub vibes: Vec<String>,
    #[serde(default = "default_count")]
    pub count: Option<usize>,
}

fn default_count() -> Option<usize> {
    Some(20)
}

/// Response body for name generation.
#[derive(Debug, Serialize)]
pub struct NameGenResponse {
    pub candidates: Vec<CandidateResponse>,
}

/// A single candidate in the response.
#[derive(Debug, Serialize)]
pub struct CandidateResponse {
    pub name: String,
    pub tagline: Option<String>,
    pub reasoning: String,
    pub status: String,
    pub domains: Vec<DomainResponse>,
    pub npm_available: Option<bool>,
    pub pypi_available: Option<bool>,
    pub github_available: Option<bool>,
    pub negative_associations: Vec<String>,
}

/// Domain availability in the response.
#[derive(Debug, Serialize)]
pub struct DomainResponse {
    pub tld: String,
    pub domain: String,
    pub available: Option<bool>,
}

impl From<NameCandidate> for CandidateResponse {
    fn from(c: NameCandidate) -> Self {
        let status = match c.validation.overall_status {
            ValidationStatus::AllClear => "all_clear",
            ValidationStatus::Partial => "partial",
            ValidationStatus::Conflicted => "conflicted",
            ValidationStatus::Pending => "pending",
        }
        .to_string();

        let domains = c
            .validation
            .domains
            .into_iter()
            .map(DomainResponse::from)
            .collect();

        CandidateResponse {
            name: c.name,
            tagline: c.tagline,
            reasoning: c.reasoning,
            status,
            domains,
            npm_available: c.validation.npm_available,
            pypi_available: c.validation.pypi_available,
            github_available: c.validation.github_available,
            negative_associations: c.validation.negative_associations,
        }
    }
}

impl From<DomainCheck> for DomainResponse {
    fn from(d: DomainCheck) -> Self {
        // Extract TLD from domain string (e.g., "myapp.com" -> "com")
        let tld = d.domain.rsplit('.').next().unwrap_or("").to_string();

        DomainResponse {
            tld,
            domain: d.domain,
            available: d.available,
        }
    }
}

/// POST /api/namegen — generate name candidates.
///
/// Tries cloud generation first if available and authenticated.
/// Falls back to local LLM provider if cloud is unavailable or user isn't signed in.
#[tracing::instrument(skip(state, req))]
pub async fn generate_names(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NameGenRequest>,
) -> Result<Json<NameGenResponse>, (StatusCode, Json<serde_json::Value>)> {
    let description = req.description;
    let vibes = req.vibes;
    let count = req.count.unwrap_or(20);

    // Try cloud generation first.
    if let Some(ref cloud) = state.cloud_client {
        if state.config.cloud.cloud_generation_enabled {
            match cloud.generate_name(&description, &vibes).await {
                Ok(cloud_resp) => {
                    let candidates = cloud_resp
                        .candidates
                        .into_iter()
                        .map(|c| CandidateResponse {
                            name: c.name,
                            tagline: c.tagline,
                            reasoning: c.reasoning,
                            status: "all_clear".to_string(),
                            domains: c
                                .domains
                                .into_iter()
                                .map(|d| {
                                    let tld = d.domain.rsplit('.').next().unwrap_or("").to_string();
                                    DomainResponse {
                                        tld,
                                        domain: d.domain,
                                        available: Some(d.available),
                                    }
                                })
                                .collect(),
                            npm_available: None,
                            pypi_available: None,
                            github_available: None,
                            negative_associations: vec![],
                        })
                        .collect();
                    return Ok(Json(NameGenResponse { candidates }));
                }
                Err(CloudError::NotAuthenticated | CloudError::AuthExpired) => {
                    tracing::info!("Cloud auth unavailable, falling back to local LLM");
                }
                Err(CloudError::InsufficientCredits {
                    required,
                    available,
                }) => {
                    return Err((
                        StatusCode::PAYMENT_REQUIRED,
                        Json(serde_json::json!({
                            "error": format!("Insufficient credits: need {required}, have {available}"),
                            "code": "insufficient_credits"
                        })),
                    ));
                }
                Err(e) => {
                    tracing::warn!("Cloud name generation failed, falling back to local: {e}");
                }
            }
        }
    }

    // Fallback: local LLM provider.
    let provider = state.llm_provider.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "No generation provider available. Sign in to Shepherd Pro or configure a local LLM."
            })),
        )
    })?;

    let input = NameGenInput {
        description,
        vibes,
        count,
    };

    let result = namegen::generate_names(provider.as_ref(), &input)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let candidates = result.candidates.into_iter().map(Into::into).collect();

    Ok(Json(NameGenResponse { candidates }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shepherd_core::namegen::{DomainCheck, NameCandidate, NameValidation, ValidationStatus};

    #[test]
    fn test_candidate_response_from_all_clear() {
        let candidate = NameCandidate {
            name: "TestApp".to_string(),
            tagline: Some("A test application".to_string()),
            reasoning: "Great name".to_string(),
            validation: NameValidation {
                domains: vec![DomainCheck {
                    domain: "testapp.com".to_string(),
                    available: Some(true),
                    error: None,
                }],
                npm_available: Some(true),
                pypi_available: Some(true),
                github_available: Some(true),
                negative_associations: Vec::new(),
                overall_status: ValidationStatus::AllClear,
            },
        };

        let response: CandidateResponse = candidate.into();
        assert_eq!(response.name, "TestApp");
        assert_eq!(response.tagline, Some("A test application".to_string()));
        assert_eq!(response.status, "all_clear");
        assert_eq!(response.domains.len(), 1);
        assert_eq!(response.domains[0].tld, "com");
        assert_eq!(response.domains[0].domain, "testapp.com");
        assert_eq!(response.domains[0].available, Some(true));
        assert_eq!(response.npm_available, Some(true));
        assert!(response.negative_associations.is_empty());
    }

    #[test]
    fn test_candidate_response_from_conflicted() {
        let candidate = NameCandidate {
            name: "BadName".to_string(),
            tagline: None,
            reasoning: "Not great".to_string(),
            validation: NameValidation {
                domains: Vec::new(),
                npm_available: None,
                pypi_available: None,
                github_available: None,
                negative_associations: vec!["offensive meaning".to_string()],
                overall_status: ValidationStatus::Conflicted,
            },
        };

        let response: CandidateResponse = candidate.into();
        assert_eq!(response.status, "conflicted");
        assert_eq!(response.negative_associations, vec!["offensive meaning"]);
    }

    #[test]
    fn test_candidate_response_status_mapping() {
        let make = |status: ValidationStatus| -> CandidateResponse {
            NameCandidate {
                name: "x".to_string(),
                tagline: None,
                reasoning: String::new(),
                validation: NameValidation {
                    overall_status: status,
                    ..Default::default()
                },
            }
            .into()
        };

        assert_eq!(make(ValidationStatus::AllClear).status, "all_clear");
        assert_eq!(make(ValidationStatus::Partial).status, "partial");
        assert_eq!(make(ValidationStatus::Conflicted).status, "conflicted");
        assert_eq!(make(ValidationStatus::Pending).status, "pending");
    }

    #[test]
    fn test_domain_response_tld_extraction() {
        let check = DomainCheck {
            domain: "myapp.dev".to_string(),
            available: Some(false),
            error: None,
        };

        let response: DomainResponse = check.into();
        assert_eq!(response.tld, "dev");
        assert_eq!(response.domain, "myapp.dev");
        assert_eq!(response.available, Some(false));
    }

    #[test]
    fn cloud_candidate_response_has_default_status() {
        // Cloud candidates don't have validation details — they default to "all_clear"
        let response = CandidateResponse {
            name: "CloudApp".to_string(),
            tagline: Some("From the cloud".to_string()),
            reasoning: "Good name".to_string(),
            status: "all_clear".to_string(),
            domains: vec![DomainResponse {
                tld: "com".to_string(),
                domain: "cloudapp.com".to_string(),
                available: Some(true),
            }],
            npm_available: None,
            pypi_available: None,
            github_available: None,
            negative_associations: vec![],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "all_clear");
        assert!(json["npm_available"].is_null());
        assert!(json["negative_associations"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_domain_response_io_tld() {
        let check = DomainCheck {
            domain: "cool.io".to_string(),
            available: None,
            error: Some("timeout".to_string()),
        };

        let response: DomainResponse = check.into();
        assert_eq!(response.tld, "io");
        assert_eq!(response.available, None);
    }

    #[test]
    fn default_count_is_20() {
        assert_eq!(default_count(), Some(20));
    }

    #[test]
    fn namegen_request_deserialize_defaults() {
        let json = r#"{"description": "A task manager"}"#;
        let req: NameGenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.description, "A task manager");
        assert!(req.vibes.is_empty());
        assert_eq!(req.count, Some(20));
    }

    #[test]
    fn namegen_request_deserialize_full() {
        let json = r#"{
            "description": "A task manager",
            "vibes": ["modern", "clean"],
            "count": 10
        }"#;
        let req: NameGenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.description, "A task manager");
        assert_eq!(req.vibes, vec!["modern", "clean"]);
        assert_eq!(req.count, Some(10));
    }

    #[test]
    fn namegen_response_serialize() {
        let resp = NameGenResponse {
            candidates: vec![CandidateResponse {
                name: "TaskFlow".to_string(),
                tagline: Some("Flow through your tasks".to_string()),
                reasoning: "Combines task + flow".to_string(),
                status: "all_clear".to_string(),
                domains: vec![DomainResponse {
                    tld: "com".to_string(),
                    domain: "taskflow.com".to_string(),
                    available: Some(true),
                }],
                npm_available: Some(true),
                pypi_available: None,
                github_available: Some(false),
                negative_associations: vec![],
            }],
        };
        let json = serde_json::to_value(&resp).unwrap();
        let candidates = json["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0]["name"], "TaskFlow");
        assert_eq!(candidates[0]["domains"].as_array().unwrap().len(), 1);
        assert!(candidates[0]["pypi_available"].is_null());
    }

    #[test]
    fn domain_response_serialize() {
        let resp = DomainResponse {
            tld: "dev".to_string(),
            domain: "myapp.dev".to_string(),
            available: Some(true),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["tld"], "dev");
        assert_eq!(json["domain"], "myapp.dev");
        assert!(json["available"].as_bool().unwrap());
    }

    #[test]
    fn domain_response_no_tld_dots() {
        // Edge case: domain without a dot
        let check = DomainCheck {
            domain: "localhost".to_string(),
            available: None,
            error: None,
        };
        let response: DomainResponse = check.into();
        assert_eq!(response.tld, "localhost");
        assert_eq!(response.domain, "localhost");
    }
}
