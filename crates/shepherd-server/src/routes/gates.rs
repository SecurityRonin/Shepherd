use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde::Serialize;
use shepherd_core::db::queries;
use shepherd_core::gates::{self, GateConfig, GateType};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct GateResultResponse {
    pub gate_name: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
    pub gate_type: String,
}

fn gate_type_to_string(gt: &GateType) -> String {
    match gt {
        GateType::Lint => "lint".to_string(),
        GateType::Format => "format".to_string(),
        GateType::TypeCheck => "type_check".to_string(),
        GateType::Test => "test".to_string(),
        GateType::Security => "security".to_string(),
        GateType::Custom => "custom".to_string(),
    }
}

#[tracing::instrument(skip(state))]
pub async fn run_task_gates(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<i64>,
) -> Result<Json<Vec<GateResultResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let db = state.db.lock().await;
    let task = queries::get_task(&db, task_id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Task not found: {}", e) })),
        )
    })?;
    drop(db);

    let repo_path = std::path::Path::new(&task.repo_path);
    let config = GateConfig::default();

    let results = gates::run_gates(repo_path, &config).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Gate execution failed: {}", e) })),
        )
    })?;

    let responses: Vec<GateResultResponse> = results
        .into_iter()
        .map(|r| GateResultResponse {
            gate_name: r.gate_name,
            passed: r.passed,
            output: r.output,
            duration_ms: r.duration_ms,
            gate_type: gate_type_to_string(&r.gate_type),
        })
        .collect();

    Ok(Json(responses))
}
