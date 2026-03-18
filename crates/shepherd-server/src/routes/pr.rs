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

#[cfg(test)]
mod tests {
    use super::*;
    use shepherd_core::pr::StepStatus;

    #[test]
    fn step_status_to_string_all_variants() {
        assert_eq!(step_status_to_string(&StepStatus::Pending), "pending");
        assert_eq!(step_status_to_string(&StepStatus::Running), "running");
        assert_eq!(step_status_to_string(&StepStatus::Passed), "passed");
        assert_eq!(step_status_to_string(&StepStatus::Failed), "failed");
        assert_eq!(step_status_to_string(&StepStatus::Skipped), "skipped");
    }

    #[test]
    fn default_base_branch_is_main() {
        assert_eq!(default_base_branch(), "main");
    }

    #[test]
    fn default_true_is_true() {
        assert!(default_true());
    }

    #[test]
    fn create_pr_request_deserialize_defaults() {
        let json = r#"{}"#;
        let req: CreatePrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.base_branch, "main");
        assert!(req.auto_commit_message);
        assert!(req.run_gates);
    }

    #[test]
    fn create_pr_request_deserialize_custom() {
        let json = r#"{
            "base_branch": "develop",
            "auto_commit_message": false,
            "run_gates": false
        }"#;
        let req: CreatePrRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.base_branch, "develop");
        assert!(!req.auto_commit_message);
        assert!(!req.run_gates);
    }

    #[test]
    fn create_pr_request_clone() {
        let req = CreatePrRequest {
            base_branch: "main".to_string(),
            auto_commit_message: true,
            run_gates: false,
        };
        let cloned = req.clone();
        assert_eq!(cloned.base_branch, req.base_branch);
        assert_eq!(cloned.auto_commit_message, req.auto_commit_message);
        assert_eq!(cloned.run_gates, req.run_gates);
    }

    #[test]
    fn step_response_serialize() {
        let step = StepResponse {
            name: "commit".to_string(),
            status: "passed".to_string(),
            output: "Committed abc123".to_string(),
        };
        let json = serde_json::to_value(&step).unwrap();
        assert_eq!(json["name"], "commit");
        assert_eq!(json["status"], "passed");
        assert_eq!(json["output"], "Committed abc123");
    }

    #[test]
    fn pr_response_serialize_success() {
        let resp = PrResponse {
            success: true,
            pr_url: Some("https://github.com/org/repo/pull/42".to_string()),
            steps: vec![
                StepResponse {
                    name: "stage".to_string(),
                    status: step_status_to_string(&StepStatus::Passed),
                    output: "staged files".to_string(),
                },
                StepResponse {
                    name: "push".to_string(),
                    status: step_status_to_string(&StepStatus::Passed),
                    output: "pushed".to_string(),
                },
            ],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert_eq!(json["pr_url"], "https://github.com/org/repo/pull/42");
        assert_eq!(json["steps"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn pr_response_serialize_failure() {
        let resp = PrResponse {
            success: false,
            pr_url: None,
            steps: vec![StepResponse {
                name: "gates".to_string(),
                status: step_status_to_string(&StepStatus::Failed),
                output: "lint failed".to_string(),
            }],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(!json["success"].as_bool().unwrap());
        assert!(json["pr_url"].is_null());
    }

    #[test]
    fn pr_response_clone() {
        let resp = PrResponse {
            success: true,
            pr_url: Some("https://example.com/pr/1".to_string()),
            steps: vec![],
        };
        let cloned = resp.clone();
        assert_eq!(cloned.success, resp.success);
        assert_eq!(cloned.pr_url, resp.pr_url);
    }
}
