pub mod commit;
pub mod github;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::gates;
use crate::llm::LlmProvider;

/// Input for the one-click PR pipeline.
#[derive(Debug, Clone)]
pub struct PrInput {
    pub task_title: String,
    pub branch: String,
    pub base_branch: String,
    pub worktree_path: String,
    pub auto_commit_message: bool,
    pub edited_commit_message: Option<String>,
    pub run_gates: bool,
    pub cleanup_worktree: bool,
}

/// Result of the full PR pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrResult {
    pub steps: Vec<PipelineStep>,
    pub pr_url: Option<String>,
    pub success: bool,
}

/// A single step in the PR pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub name: String,
    pub status: StepStatus,
    pub output: String,
}

/// Status of a pipeline step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

/// Run the full PR pipeline: stage, commit, rebase, gates, push, create PR, cleanup.
pub async fn create_pr<F>(
    input: &PrInput,
    llm: Option<&dyn LlmProvider>,
    on_step: F,
) -> Result<PrResult>
where
    F: Fn(&PipelineStep),
{
    let project_dir = Path::new(&input.worktree_path);
    let mut steps = Vec::new();
    let mut pr_url = None;

    // Step 1: Stage all changes
    let stage_step = run_step("Stage Changes", || async {
        github::git_stage_all(project_dir).await
    })
    .await;
    on_step(&stage_step);
    let stage_ok = stage_step.status == StepStatus::Passed;
    steps.push(stage_step);
    if !stage_ok {
        return Ok(PrResult {
            steps,
            pr_url: None,
            success: false,
        });
    }

    // Step 2: Generate commit message
    let commit_msg = if let Some(edited) = &input.edited_commit_message {
        let step = PipelineStep {
            name: "Generate Commit Message".into(),
            status: StepStatus::Passed,
            output: edited.clone(),
        };
        on_step(&step);
        steps.push(step);
        edited.clone()
    } else if input.auto_commit_message {
        let diff = github::git_diff_staged(project_dir).await.unwrap_or_default();
        let msg = if let Some(provider) = llm {
            commit::generate_commit_message(provider, &diff, &input.task_title)
                .await
                .unwrap_or_else(|_| format!("feat: {}", input.task_title))
        } else {
            format!("feat: {}", input.task_title)
        };
        let step = PipelineStep {
            name: "Generate Commit Message".into(),
            status: StepStatus::Passed,
            output: msg.clone(),
        };
        on_step(&step);
        steps.push(step);
        msg
    } else {
        let msg = format!("feat: {}", input.task_title);
        let step = PipelineStep {
            name: "Generate Commit Message".into(),
            status: StepStatus::Skipped,
            output: msg.clone(),
        };
        on_step(&step);
        steps.push(step);
        msg
    };

    // Step 3: Commit
    let commit_step = run_step("Commit", || async {
        github::git_commit(project_dir, &commit_msg).await
    })
    .await;
    on_step(&commit_step);
    let commit_ok = commit_step.status == StepStatus::Passed;
    steps.push(commit_step);
    if !commit_ok {
        return Ok(PrResult {
            steps,
            pr_url: None,
            success: false,
        });
    }

    // Step 4: Rebase on base branch
    let rebase_step = run_step("Rebase", || async {
        github::git_rebase(project_dir, &input.base_branch).await
    })
    .await;
    on_step(&rebase_step);
    let rebase_ok = rebase_step.status == StepStatus::Passed;
    steps.push(rebase_step);
    if !rebase_ok {
        // Abort rebase on failure
        let _ = github::git_rebase_abort(project_dir).await;
        return Ok(PrResult {
            steps,
            pr_url: None,
            success: false,
        });
    }

    // Step 5: Quality gates
    if input.run_gates {
        let gate_config = gates::GateConfig::default();
        let gate_results = gates::run_gates(project_dir, &gate_config).await?;
        let gates_passed = gates::all_gates_passed(&gate_results);
        let gate_output = gate_results
            .iter()
            .map(|r| {
                format!(
                    "{}: {}",
                    r.gate_name,
                    if r.passed { "PASS" } else { "FAIL" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let gate_step = PipelineStep {
            name: "Quality Gates".into(),
            status: if gates_passed {
                StepStatus::Passed
            } else {
                StepStatus::Failed
            },
            output: gate_output,
        };
        on_step(&gate_step);
        steps.push(gate_step);
        if !gates_passed {
            return Ok(PrResult {
                steps,
                pr_url: None,
                success: false,
            });
        }
    } else {
        let gate_step = PipelineStep {
            name: "Quality Gates".into(),
            status: StepStatus::Skipped,
            output: "Gates skipped".into(),
        };
        on_step(&gate_step);
        steps.push(gate_step);
    }

    // Step 6: Push
    let push_step = run_step("Push", || async {
        github::git_push(project_dir, &input.branch).await
    })
    .await;
    on_step(&push_step);
    let push_ok = push_step.status == StepStatus::Passed;
    steps.push(push_step);
    if !push_ok {
        return Ok(PrResult {
            steps,
            pr_url: None,
            success: false,
        });
    }

    // Step 7: Create PR
    let diff_stat = github::git_diff_staged(project_dir)
        .await
        .unwrap_or_default();
    let pr_body = github::build_pr_body(&input.task_title, &diff_stat, &steps);
    let pr_step_result = github::gh_create_pr(
        project_dir,
        &input.task_title,
        &pr_body,
        &input.base_branch,
    )
    .await;
    let pr_step = match &pr_step_result {
        Ok(url) => {
            pr_url = Some(url.clone());
            PipelineStep {
                name: "Create PR".into(),
                status: StepStatus::Passed,
                output: url.clone(),
            }
        }
        Err(e) => PipelineStep {
            name: "Create PR".into(),
            status: StepStatus::Failed,
            output: format!("Failed to create PR: {e}"),
        },
    };
    on_step(&pr_step);
    let pr_ok = pr_step.status == StepStatus::Passed;
    steps.push(pr_step);

    // Step 8: Cleanup worktree
    if input.cleanup_worktree {
        let cleanup_step = run_step("Cleanup Worktree", || async {
            github::git_remove_worktree(project_dir).await
        })
        .await;
        on_step(&cleanup_step);
        steps.push(cleanup_step);
    } else {
        let cleanup_step = PipelineStep {
            name: "Cleanup Worktree".into(),
            status: StepStatus::Skipped,
            output: "Worktree cleanup skipped".into(),
        };
        on_step(&cleanup_step);
        steps.push(cleanup_step);
    }

    Ok(PrResult {
        steps,
        pr_url,
        success: pr_ok,
    })
}

async fn run_step<F, Fut>(name: &str, f: F) -> PipelineStep
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    match f().await {
        Ok(output) => PipelineStep {
            name: name.into(),
            status: StepStatus::Passed,
            output,
        },
        Err(e) => PipelineStep {
            name: name.into(),
            status: StepStatus::Failed,
            output: format!("{e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_status_serde() {
        let passed = serde_json::to_string(&StepStatus::Passed).unwrap();
        assert_eq!(passed, "\"passed\"");

        let failed = serde_json::to_string(&StepStatus::Failed).unwrap();
        assert_eq!(failed, "\"failed\"");

        let skipped = serde_json::to_string(&StepStatus::Skipped).unwrap();
        assert_eq!(skipped, "\"skipped\"");

        let pending = serde_json::to_string(&StepStatus::Pending).unwrap();
        assert_eq!(pending, "\"pending\"");

        let running = serde_json::to_string(&StepStatus::Running).unwrap();
        assert_eq!(running, "\"running\"");

        // Roundtrip
        let from_str: StepStatus = serde_json::from_str("\"passed\"").unwrap();
        assert_eq!(from_str, StepStatus::Passed);
    }

    #[test]
    fn pr_result_default_failure() {
        let result = PrResult {
            steps: vec![],
            pr_url: None,
            success: false,
        };
        assert!(!result.success);
        assert!(result.pr_url.is_none());
        assert!(result.steps.is_empty());
    }
}
