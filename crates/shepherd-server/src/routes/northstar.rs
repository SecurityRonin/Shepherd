use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
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
pub async fn execute_phase(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecutePhaseRequest>,
) -> Result<Json<ExecutePhaseResponse>, (StatusCode, Json<serde_json::Value>)> {
    let provider = state.llm_provider.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "LLM provider not configured"
            })),
        )
    })?;

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
}
