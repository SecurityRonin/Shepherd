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

#[cfg(test)]
mod tests {
    use super::*;
    use shepherd_core::gates::GateType;

    #[test]
    fn gate_type_to_string_all_variants() {
        assert_eq!(gate_type_to_string(&GateType::Lint), "lint");
        assert_eq!(gate_type_to_string(&GateType::Format), "format");
        assert_eq!(gate_type_to_string(&GateType::TypeCheck), "type_check");
        assert_eq!(gate_type_to_string(&GateType::Test), "test");
        assert_eq!(gate_type_to_string(&GateType::Security), "security");
        assert_eq!(gate_type_to_string(&GateType::Custom), "custom");
    }

    #[test]
    fn gate_result_response_serialize() {
        let resp = GateResultResponse {
            gate_name: "clippy".to_string(),
            passed: true,
            output: "All checks passed".to_string(),
            duration_ms: 1234,
            gate_type: "lint".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["gate_name"], "clippy");
        assert!(json["passed"].as_bool().unwrap());
        assert_eq!(json["output"], "All checks passed");
        assert_eq!(json["duration_ms"], 1234);
        assert_eq!(json["gate_type"], "lint");
    }

    #[test]
    fn gate_result_response_failed_gate() {
        let resp = GateResultResponse {
            gate_name: "tests".to_string(),
            passed: false,
            output: "3 tests failed".to_string(),
            duration_ms: 5678,
            gate_type: "test".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(!json["passed"].as_bool().unwrap());
        assert_eq!(json["gate_type"], "test");
    }

    #[test]
    fn gate_result_response_clone() {
        let resp = GateResultResponse {
            gate_name: "fmt".to_string(),
            passed: true,
            output: String::new(),
            duration_ms: 100,
            gate_type: "format".to_string(),
        };
        let cloned = resp.clone();
        assert_eq!(cloned.gate_name, resp.gate_name);
        assert_eq!(cloned.passed, resp.passed);
        assert_eq!(cloned.duration_ms, resp.duration_ms);
    }

    #[test]
    fn gate_result_response_serialize_vec() {
        let results = vec![
            GateResultResponse {
                gate_name: "lint".to_string(),
                passed: true,
                output: "ok".to_string(),
                duration_ms: 100,
                gate_type: gate_type_to_string(&GateType::Lint),
            },
            GateResultResponse {
                gate_name: "security".to_string(),
                passed: false,
                output: "vulnerability found".to_string(),
                duration_ms: 200,
                gate_type: gate_type_to_string(&GateType::Security),
            },
        ];
        let json = serde_json::to_value(&results).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert!(arr[0]["passed"].as_bool().unwrap());
        assert!(!arr[1]["passed"].as_bool().unwrap());
        assert_eq!(arr[1]["gate_type"], "security");
    }
}
