use axum::{extract::Path, extract::State, http::StatusCode, Json};
use shepherd_core::observability::{self, SpendingSummary, TaskMetrics};
use std::sync::Arc;

use crate::state::AppState;

/// GET /api/metrics — return aggregate spending summary.
#[tracing::instrument(skip(state))]
pub async fn spending_summary(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SpendingSummary>, (StatusCode, Json<serde_json::Value>)> {
    let db = state.db.lock().await;
    let summary = observability::store::get_spending_summary(&db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )
    })?;
    Ok(Json(summary))
}

/// GET /api/metrics/:task_id — return metrics for a specific task.
#[tracing::instrument(skip(state))]
pub async fn task_metrics(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<i64>,
) -> Result<Json<TaskMetrics>, (StatusCode, Json<serde_json::Value>)> {
    let db = state.db.lock().await;
    let metrics = observability::store::get_task_metrics(&db, task_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )
    })?;
    match metrics {
        Some(m) => Ok(Json(m)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("No metrics found for task {}", task_id) })),
        )),
    }
}

#[cfg(test)]
mod tests {
    use shepherd_core::observability::{
        AgentSpending, ModelSpending, SpendingSummary, TaskMetrics,
    };

    #[test]
    fn spending_summary_response_serialize() {
        let summary = SpendingSummary {
            total_cost_usd: 1.25,
            total_tokens: 50_000,
            total_tasks: 3,
            total_llm_calls: 10,
            by_agent: vec![AgentSpending {
                agent_id: "claude".to_string(),
                total_cost_usd: 1.25,
                total_tokens: 50_000,
                task_count: 3,
            }],
            by_model: vec![ModelSpending {
                model_id: "claude-sonnet-4".to_string(),
                total_cost_usd: 1.25,
                total_tokens: 50_000,
                call_count: 10,
            }],
        };

        let json = serde_json::to_value(&summary).expect("serialize SpendingSummary");
        assert_eq!(json["total_cost_usd"], 1.25);
        assert_eq!(json["total_tokens"], 50_000);
        assert_eq!(json["total_tasks"], 3);
        assert_eq!(json["total_llm_calls"], 10);

        let agents = json["by_agent"].as_array().expect("by_agent is array");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0]["agent_id"], "claude");
        assert_eq!(agents[0]["task_count"], 3);

        let models = json["by_model"].as_array().expect("by_model is array");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0]["model_id"], "claude-sonnet-4");
        assert_eq!(models[0]["call_count"], 10);
    }

    #[test]
    fn task_metrics_response_serialize() {
        let metrics = TaskMetrics {
            task_id: 42,
            agent_id: "codex".to_string(),
            model_id: "gpt-4o".to_string(),
            total_input_tokens: 10_000,
            total_output_tokens: 5_000,
            total_tokens: 15_000,
            total_cost_usd: 0.75,
            llm_calls: 4,
            duration_secs: Some(120.5),
            status: "done".to_string(),
            created_at: "2026-03-20T00:00:00Z".to_string(),
            updated_at: "2026-03-20T00:05:00Z".to_string(),
        };

        let json = serde_json::to_value(&metrics).expect("serialize TaskMetrics");
        assert_eq!(json["task_id"], 42);
        assert_eq!(json["agent_id"], "codex");
        assert_eq!(json["model_id"], "gpt-4o");
        assert_eq!(json["total_input_tokens"], 10_000);
        assert_eq!(json["total_output_tokens"], 5_000);
        assert_eq!(json["total_tokens"], 15_000);
        assert_eq!(json["total_cost_usd"], 0.75);
        assert_eq!(json["llm_calls"], 4);
        assert_eq!(json["duration_secs"], 120.5);
        assert_eq!(json["status"], "done");
        assert_eq!(json["created_at"], "2026-03-20T00:00:00Z");
        assert_eq!(json["updated_at"], "2026-03-20T00:05:00Z");
    }

    #[test]
    fn task_metrics_not_found_response() {
        let task_id = 999;
        let body = serde_json::json!({ "error": format!("No metrics found for task {}", task_id) });

        assert_eq!(
            body["error"].as_str().unwrap(),
            "No metrics found for task 999"
        );
        // Verify the shape: single "error" key with a string value
        let map = body.as_object().expect("response is a JSON object");
        assert_eq!(map.len(), 1, "response should have exactly one key");
        assert!(map.contains_key("error"), "key must be 'error'");
        assert!(map["error"].is_string(), "error value must be a string");
    }
}
