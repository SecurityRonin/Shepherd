use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

use super::PipelineStep;

/// Run a git command in the given directory, returning stdout on success.
async fn run_git(project_dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_dir)
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim())
    }
}

/// Stage all changes (git add -A).
pub async fn git_stage_all(project_dir: &Path) -> Result<String> {
    run_git(project_dir, &["add", "-A"]).await?;
    Ok("All changes staged".into())
}

/// Get the staged diff (git diff --cached --stat).
pub async fn git_diff_staged(project_dir: &Path) -> Result<String> {
    run_git(project_dir, &["diff", "--cached", "--stat"]).await
}

/// Commit staged changes with the given message.
pub async fn git_commit(project_dir: &Path, message: &str) -> Result<String> {
    run_git(project_dir, &["commit", "-m", message]).await
}

/// Rebase the current branch onto the given base branch.
pub async fn git_rebase(project_dir: &Path, base_branch: &str) -> Result<String> {
    run_git(project_dir, &["rebase", base_branch]).await
}

/// Abort a rebase in progress.
pub async fn git_rebase_abort(project_dir: &Path) -> Result<String> {
    run_git(project_dir, &["rebase", "--abort"]).await
}

/// Remove the worktree at the given path.
pub async fn git_remove_worktree(project_dir: &Path) -> Result<String> {
    // Use the parent repo to remove this worktree
    run_git(
        project_dir,
        &["worktree", "remove", "--force", "."],
    )
    .await
}

/// Push the branch to origin.
pub async fn git_push(project_dir: &Path, branch: &str) -> Result<String> {
    run_git(project_dir, &["push", "-u", "origin", branch]).await
}

/// Create a PR using the gh CLI.
pub async fn gh_create_pr(
    project_dir: &Path,
    title: &str,
    body: &str,
    base_branch: &str,
) -> Result<String> {
    let output = Command::new("gh")
        .args([
            "pr",
            "create",
            "--title",
            title,
            "--body",
            body,
            "--base",
            base_branch,
        ])
        .current_dir(project_dir)
        .output()
        .await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh pr create failed: {}", stderr.trim())
    }
}

/// Build the PR body markdown with pipeline results.
pub fn build_pr_body(
    task_title: &str,
    diff_stat: &str,
    steps: &[PipelineStep],
) -> String {
    let mut body = String::new();

    body.push_str("## Summary\n\n");
    body.push_str(&format!("Task: {task_title}\n\n"));

    body.push_str("## Changes\n\n");
    body.push_str("```\n");
    body.push_str(diff_stat);
    body.push_str("\n```\n\n");

    body.push_str("## Pipeline Results\n\n");
    body.push_str("| Step | Status |\n");
    body.push_str("|------|--------|\n");
    for step in steps {
        let status_str = match &step.status {
            super::StepStatus::Passed => "PASS",
            super::StepStatus::Failed => "FAIL",
            super::StepStatus::Skipped => "SKIP",
            super::StepStatus::Pending => "PENDING",
            super::StepStatus::Running => "RUNNING",
        };
        body.push_str(&format!("| {} | {status_str} |\n", step.name));
    }
    body.push('\n');

    body.push_str("---\n\n");
    body.push_str("Created by Shepherd\n");

    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::StepStatus;

    #[test]
    fn build_pr_body_test() {
        let steps = vec![
            PipelineStep {
                name: "Stage Changes".into(),
                status: StepStatus::Passed,
                output: "staged".into(),
            },
            PipelineStep {
                name: "Commit".into(),
                status: StepStatus::Passed,
                output: "committed".into(),
            },
            PipelineStep {
                name: "Quality Gates".into(),
                status: StepStatus::Failed,
                output: "lint failed".into(),
            },
            PipelineStep {
                name: "Cleanup".into(),
                status: StepStatus::Skipped,
                output: "skipped".into(),
            },
        ];

        let body = build_pr_body(
            "Add new feature",
            "3 files changed, 100 insertions(+), 20 deletions(-)",
            &steps,
        );

        // Check task title appears
        assert!(body.contains("Add new feature"));
        // Check diff stat
        assert!(body.contains("3 files changed"));
        // Check step names
        assert!(body.contains("Stage Changes"));
        assert!(body.contains("Commit"));
        assert!(body.contains("Quality Gates"));
        assert!(body.contains("Cleanup"));
        // Check status values
        assert!(body.contains("PASS"));
        assert!(body.contains("FAIL"));
        assert!(body.contains("SKIP"));
        // Check footer
        assert!(body.contains("Created by Shepherd"));
    }

    #[test]
    fn build_pr_body_all_statuses() {
        let steps = vec![
            PipelineStep {
                name: "Stage".into(),
                status: StepStatus::Pending,
                output: "pending".into(),
            },
            PipelineStep {
                name: "Build".into(),
                status: StepStatus::Running,
                output: "running".into(),
            },
            PipelineStep {
                name: "Test".into(),
                status: StepStatus::Passed,
                output: "passed".into(),
            },
            PipelineStep {
                name: "Lint".into(),
                status: StepStatus::Failed,
                output: "failed".into(),
            },
            PipelineStep {
                name: "Deploy".into(),
                status: StepStatus::Skipped,
                output: "skipped".into(),
            },
        ];

        let body = build_pr_body("Multi-status test", "5 files changed", &steps);
        assert!(body.contains("PENDING"));
        assert!(body.contains("RUNNING"));
        assert!(body.contains("PASS"));
        assert!(body.contains("FAIL"));
        assert!(body.contains("SKIP"));
        assert!(body.contains("| Stage | PENDING |"));
        assert!(body.contains("| Build | RUNNING |"));
    }

    #[test]
    fn build_pr_body_empty_steps() {
        let body = build_pr_body("Empty", "", &[]);
        assert!(body.contains("Empty"));
        assert!(body.contains("Pipeline Results"));
        assert!(body.contains("Created by Shepherd"));
    }

    #[test]
    fn build_pr_body_sections_present() {
        let body = build_pr_body("Test", "1 file", &[]);
        assert!(body.contains("## Summary"));
        assert!(body.contains("## Changes"));
        assert!(body.contains("## Pipeline Results"));
        assert!(body.contains("| Step | Status |"));
    }

    #[tokio::test]
    async fn git_stage_all_in_nonexistent_dir() {
        let result = git_stage_all(std::path::Path::new("/nonexistent/dir")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn git_diff_staged_in_nonexistent_dir() {
        let result = git_diff_staged(std::path::Path::new("/nonexistent/dir")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn git_commit_in_nonexistent_dir() {
        let result = git_commit(std::path::Path::new("/nonexistent/dir"), "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn git_rebase_in_nonexistent_dir() {
        let result = git_rebase(std::path::Path::new("/nonexistent/dir"), "main").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn git_rebase_abort_in_nonexistent_dir() {
        let result = git_rebase_abort(std::path::Path::new("/nonexistent/dir")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn git_push_in_nonexistent_dir() {
        let result = git_push(std::path::Path::new("/nonexistent/dir"), "main").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn git_stage_all_in_temp_repo() {
        let tmp = tempfile::tempdir().unwrap();
        // Initialize a git repo
        tokio::process::Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .await
            .unwrap();
        tokio::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(tmp.path())
            .output()
            .await
            .unwrap();
        tokio::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(tmp.path())
            .output()
            .await
            .unwrap();

        // Create a file
        std::fs::write(tmp.path().join("test.txt"), "hello").unwrap();

        let result = git_stage_all(tmp.path()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "All changes staged");
    }

    #[tokio::test]
    async fn git_diff_staged_in_temp_repo() {
        let tmp = tempfile::tempdir().unwrap();
        tokio::process::Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .await
            .unwrap();

        std::fs::write(tmp.path().join("test.txt"), "hello").unwrap();
        git_stage_all(tmp.path()).await.unwrap();

        let result = git_diff_staged(tmp.path()).await;
        assert!(result.is_ok());
        // New file should appear in diff stat
        let diff = result.unwrap();
        assert!(diff.contains("test.txt") || diff.is_empty());
    }
}
