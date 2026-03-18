use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use shepherd_core::cloud::CloudError;
use shepherd_core::northstar::phases::PHASES;
use std::sync::Arc;

use crate::state::AppState;

// ── Request / Response types ─────────────────────────────────────────

/// A phase definition returned by the list endpoint.
#[derive(Debug, Serialize)]
pub struct PhaseInfo {
    pub id: u8,
    pub name: String,
    pub description: String,
    pub document_count: usize,
}

/// Response for listing all phases.
#[derive(Debug, Serialize)]
pub struct ListPhasesResponse {
    pub phases: Vec<PhaseInfo>,
    pub total: usize,
}

/// Request body for executing a single phase.
#[derive(Debug, Deserialize)]
pub struct ExecutePhaseRequest {
    pub product_name: String,
    pub product_description: String,
    pub phase_id: u8,
    #[serde(default)]
    pub previous_context: Option<String>,
}

/// Response from executing a single phase.
#[derive(Debug, Serialize)]
pub struct ExecutePhaseResponse {
    pub phase_id: u8,
    pub phase_name: String,
    pub status: String,
    pub output: String,
    pub documents: Vec<DocumentResponse>,
}

/// A generated document in the response.
#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    pub title: String,
    pub filename: String,
    pub doc_type: String,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// GET /api/northstar/phases — list all analysis phases.
#[tracing::instrument]
pub async fn list_phases() -> Json<ListPhasesResponse> {
    let phases: Vec<PhaseInfo> = PHASES
        .iter()
        .map(|p| PhaseInfo {
            id: p.id,
            name: p.name.to_string(),
            description: p.description.to_string(),
            document_count: p.output_documents.len(),
        })
        .collect();

    let total = phases.len();

    Json(ListPhasesResponse { phases, total })
}

/// POST /api/northstar/phase — execute a single analysis phase.
///
/// Tries cloud generation first if available and authenticated.
/// Falls back to local LLM provider if cloud is unavailable or user isn't signed in.
#[tracing::instrument(skip(state, req))]
pub async fn execute_phase(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecutePhaseRequest>,
) -> Result<Json<ExecutePhaseResponse>, (StatusCode, Json<serde_json::Value>)> {
    let phase = PHASES
        .iter()
        .find(|p| p.id == req.phase_id)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Unknown phase_id: {}", req.phase_id)
                })),
            )
        })?;

    // Try cloud generation first.
    if let Some(ref cloud) = state.cloud_client {
        if state.config.cloud.cloud_generation_enabled {
            let context = serde_json::json!({
                "product_name": req.product_name,
                "product_description": req.product_description,
                "previous_context": req.previous_context,
            });

            match cloud.generate_northstar(phase.name, context).await {
                Ok(cloud_resp) => {
                    let output = cloud_resp
                        .result
                        .get("output")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let documents = cloud_resp
                        .result
                        .get("documents")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|d| {
                                    Some(DocumentResponse {
                                        title: d.get("title")?.as_str()?.to_string(),
                                        filename: d.get("filename")?.as_str()?.to_string(),
                                        doc_type: d
                                            .get("doc_type")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("markdown")
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    return Ok(Json(ExecutePhaseResponse {
                        phase_id: req.phase_id,
                        phase_name: phase.name.to_string(),
                        status: "completed".to_string(),
                        output,
                        documents,
                    }));
                }
                Err(CloudError::NotAuthenticated | CloudError::AuthExpired) => {
                    tracing::info!("Cloud auth unavailable, falling back to local LLM");
                }
                Err(CloudError::InsufficientCredits { required, available }) => {
                    return Err((
                        StatusCode::PAYMENT_REQUIRED,
                        Json(serde_json::json!({
                            "error": format!("Insufficient credits: need {required}, have {available}"),
                            "code": "insufficient_credits"
                        })),
                    ));
                }
                Err(e) => {
                    tracing::warn!(
                        "Cloud northstar generation failed, falling back to local: {e}"
                    );
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

    let result = shepherd_core::northstar::phases::execute_phase(
        provider.as_ref(),
        phase,
        &req.product_name,
        &req.product_description,
        req.previous_context.as_deref(),
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Phase execution failed: {e}")
            })),
        )
    })?;

    let documents = result
        .documents
        .iter()
        .map(|d| DocumentResponse {
            title: d.title.clone(),
            filename: d.filename.clone(),
            doc_type: d.doc_type.clone(),
        })
        .collect();

    let status = serde_json::to_value(&result.status)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "completed".to_string());

    Ok(Json(ExecutePhaseResponse {
        phase_id: result.phase_id,
        phase_name: result.phase_name,
        status,
        output: result.output,
        documents,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_phases_returns_13() {
        let response = futures::executor::block_on(list_phases());
        assert_eq!(response.total, 13);
        assert_eq!(response.phases.len(), 13);
    }

    #[test]
    fn phase_info_ids_sequential() {
        let response = futures::executor::block_on(list_phases());
        for (i, phase) in response.phases.iter().enumerate() {
            assert_eq!(phase.id, (i + 1) as u8);
        }
    }

    #[test]
    fn phase_info_has_documents() {
        let response = futures::executor::block_on(list_phases());
        for phase in &response.phases {
            assert!(phase.document_count > 0);
        }
    }

    #[test]
    fn execute_phase_request_deserialize() {
        let json = r#"{
            "product_name": "TestApp",
            "product_description": "A test application",
            "phase_id": 1
        }"#;

        let req: ExecutePhaseRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.product_name, "TestApp");
        assert_eq!(req.phase_id, 1);
        assert!(req.previous_context.is_none());
    }

    #[test]
    fn execute_phase_request_with_context() {
        let json = r#"{
            "product_name": "TestApp",
            "product_description": "A test application",
            "phase_id": 3,
            "previous_context": "Previous phase output"
        }"#;

        let req: ExecutePhaseRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.phase_id, 3);
        assert_eq!(
            req.previous_context,
            Some("Previous phase output".to_string())
        );
    }

    #[test]
    fn cloud_northstar_result_parsing() {
        // Simulate the JSON shape a cloud northstar response would provide
        let cloud_result = serde_json::json!({
            "output": "Product vision analysis complete.",
            "documents": [
                {
                    "title": "Product Vision",
                    "filename": "product-vision.md",
                    "doc_type": "markdown"
                },
                {
                    "title": "Market Analysis",
                    "filename": "market-analysis.md"
                }
            ]
        });

        let output = cloud_result
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(output, "Product vision analysis complete.");

        let docs: Vec<DocumentResponse> = cloud_result
            .get("documents")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|d| {
                        Some(DocumentResponse {
                            title: d.get("title")?.as_str()?.to_string(),
                            filename: d.get("filename")?.as_str()?.to_string(),
                            doc_type: d
                                .get("doc_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("markdown")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].title, "Product Vision");
        assert_eq!(docs[1].doc_type, "markdown"); // defaults when missing
    }

    #[test]
    fn cloud_northstar_empty_result() {
        // Cloud result with no output or documents
        let cloud_result = serde_json::json!({});

        let output = cloud_result
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(output, "");

        let docs: Vec<DocumentResponse> = cloud_result
            .get("documents")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|d| {
                        Some(DocumentResponse {
                            title: d.get("title")?.as_str()?.to_string(),
                            filename: d.get("filename")?.as_str()?.to_string(),
                            doc_type: d
                                .get("doc_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("markdown")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        assert!(docs.is_empty());
    }

    #[test]
    fn execute_phase_response_serialize() {
        let response = ExecutePhaseResponse {
            phase_id: 1,
            phase_name: "Product Vision".to_string(),
            status: "completed".to_string(),
            output: "Vision analysis output".to_string(),
            documents: vec![DocumentResponse {
                title: "Product Vision".to_string(),
                filename: "product-vision.md".to_string(),
                doc_type: "markdown".to_string(),
            }],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["phase_id"], 1);
        assert_eq!(json["status"], "completed");
        assert_eq!(json["documents"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn phase_info_serialize() {
        let info = PhaseInfo {
            id: 5,
            name: "Go-to-Market".to_string(),
            description: "Define GTM strategy".to_string(),
            document_count: 3,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], 5);
        assert_eq!(json["name"], "Go-to-Market");
        assert_eq!(json["document_count"], 3);
    }

    #[test]
    fn list_phases_response_serialize() {
        let resp = ListPhasesResponse {
            phases: vec![
                PhaseInfo {
                    id: 1,
                    name: "Vision".to_string(),
                    description: "Product vision".to_string(),
                    document_count: 2,
                },
                PhaseInfo {
                    id: 2,
                    name: "Research".to_string(),
                    description: "Market research".to_string(),
                    document_count: 4,
                },
            ],
            total: 2,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["total"], 2);
        assert_eq!(json["phases"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn document_response_serialize() {
        let doc = DocumentResponse {
            title: "PRD".to_string(),
            filename: "prd.md".to_string(),
            doc_type: "markdown".to_string(),
        };
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["title"], "PRD");
        assert_eq!(json["filename"], "prd.md");
        assert_eq!(json["doc_type"], "markdown");
    }

    #[test]
    fn list_phases_total_matches_len() {
        let response = futures::executor::block_on(list_phases());
        assert_eq!(response.total, response.phases.len());
    }

    #[test]
    fn list_phases_names_not_empty() {
        let response = futures::executor::block_on(list_phases());
        for phase in &response.phases {
            assert!(!phase.name.is_empty());
            assert!(!phase.description.is_empty());
        }
    }

    #[test]
    fn cloud_northstar_partial_documents() {
        // Documents missing required fields should be filtered out by filter_map
        let cloud_result = serde_json::json!({
            "output": "Analysis done.",
            "documents": [
                {
                    "title": "Valid Doc",
                    "filename": "valid.md",
                    "doc_type": "markdown"
                },
                {
                    "title": "Missing Filename"
                },
                {
                    "filename": "missing-title.md"
                }
            ]
        });

        let docs: Vec<DocumentResponse> = cloud_result
            .get("documents")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|d| {
                        Some(DocumentResponse {
                            title: d.get("title")?.as_str()?.to_string(),
                            filename: d.get("filename")?.as_str()?.to_string(),
                            doc_type: d
                                .get("doc_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("markdown")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Only the first doc has both title and filename
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].title, "Valid Doc");
    }
}
