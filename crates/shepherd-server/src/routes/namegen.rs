use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
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
        let tld = d
            .domain
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_string();

        DomainResponse {
            tld,
            domain: d.domain,
            available: d.available,
        }
    }
}

/// POST /api/namegen — generate name candidates.
#[tracing::instrument(skip(state, req))]
pub async fn generate_names(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NameGenRequest>,
) -> Result<Json<NameGenResponse>, (StatusCode, Json<serde_json::Value>)> {
    let provider = state.llm_provider.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "LLM provider not configured"
            })),
        )
    })?;

    let input = NameGenInput {
        description: req.description,
        vibes: req.vibes,
        count: req.count.unwrap_or(20),
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
}
