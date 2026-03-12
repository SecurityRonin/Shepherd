use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use shepherd_core::db::queries;
use shepherd_core::pr::{self, PrInput, StepStatus};
use std::sync::Arc;

use crate::state::AppState;

fn default_base_branch() -> String {
    "main".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePrRequest {
    #[serde(default = "default_base_branch")]
    pub base_branch: String,
    #[serde(default = "default_true")]
    pub auto_commit_message: bool,
    #[serde(default = "default_true")]
    pub run_gates: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepResponse {
    pub name: String,
    pub status: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrResponse {
    pub success: bool,
    pub pr_url: Option<String>,
    pub steps: Vec<StepResponse>,
}

fn step_status_to_string(status: &StepStatus) -> String {
    match status {
        StepStatus::Pending => "pending".to_string(),
        StepStatus::Running => "running".to_string(),
        StepStatus::Passed => "passed".to_string(),
        StepStatus::Failed => "failed".to_string(),
        StepStatus::Skipped => "skipped".to_string(),
    }
}

#[tracing::instrument(skip(state, input))]
pub async fn create_pr(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<i64>,
    Json(input): Json<CreatePrRequest>,
) -> Result<Json<PrResponse>, (StatusCode, Json<serde_json::Value>)> {
    let db = state.db.lock().await;
    let task = queries::get_task(&db, task_id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Task not found: {}", e) })),
        )
    })?;
    drop(db);

    let pr_input = PrInput {
        task_title: task.title,
        branch: task.branch,
        base_branch: input.base_branch,
        worktree_path: task.repo_path,
        auto_commit_message: input.auto_commit_message,
        edited_commit_message: None,
        run_gates: input.run_gates,
        cleanup_worktree: true,
    };

    let llm_ref = state.llm_provider.as_deref();

    let result = pr::create_pr(&pr_input, llm_ref, |_step| {})
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("PR creation failed: {}", e) })),
            )
        })?;

    let steps: Vec<StepResponse> = result
        .steps
        .iter()
        .map(|s| StepResponse {
            name: s.name.clone(),
            status: step_status_to_string(&s.status),
            output: s.output.clone(),
        })
        .collect();

    Ok(Json(PrResponse {
        success: result.success,
        pr_url: result.pr_url,
        steps,
    }))
}
