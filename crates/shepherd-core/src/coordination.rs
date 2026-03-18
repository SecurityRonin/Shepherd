//! Agent coordination — file locking, conflict detection,
//! and capability-based task routing.
//!
//! Prevents two agents from editing the same files simultaneously
//! and routes tasks to agents based on their declared capabilities.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// A file lock held by a running task.
#[derive(Debug, Clone)]
struct FileLock {
    task_id: i64,
    agent_id: String,
}

/// Manages file-level locks to prevent conflicts between concurrent agents.
#[derive(Debug, Default)]
pub struct LockManager {
    /// Map of file path → lock holder
    locks: HashMap<PathBuf, FileLock>,
}

/// Result of attempting to acquire locks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockResult {
    /// All locks acquired successfully.
    Acquired,
    /// Some files are already locked by another task.
    Conflict(Vec<FileConflict>),
}

/// A conflict between two tasks over a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileConflict {
    pub file_path: PathBuf,
    pub held_by_task: i64,
    pub held_by_agent: String,
    pub requested_by_task: i64,
}

impl LockManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempt to acquire locks on a set of files for a task.
    /// Returns Acquired if all locks were obtained, or Conflict with details.
    pub fn try_acquire(
        &mut self,
        task_id: i64,
        agent_id: &str,
        files: &[PathBuf],
    ) -> LockResult {
        // Check for conflicts first
        let mut conflicts = Vec::new();
        for file in files {
            if let Some(lock) = self.locks.get(file) {
                if lock.task_id != task_id {
                    conflicts.push(FileConflict {
                        file_path: file.clone(),
                        held_by_task: lock.task_id,
                        held_by_agent: lock.agent_id.clone(),
                        requested_by_task: task_id,
                    });
                }
            }
        }

        if !conflicts.is_empty() {
            return LockResult::Conflict(conflicts);
        }

        // No conflicts — acquire all locks
        for file in files {
            self.locks.insert(
                file.clone(),
                FileLock {
                    task_id,
                    agent_id: agent_id.to_string(),
                },
            );
        }

        LockResult::Acquired
    }

    /// Release all locks held by a task.
    pub fn release(&mut self, task_id: i64) -> usize {
        let before = self.locks.len();
        self.locks.retain(|_, lock| lock.task_id != task_id);
        before - self.locks.len()
    }

    /// Get all files locked by a specific task.
    pub fn locks_for_task(&self, task_id: i64) -> Vec<PathBuf> {
        self.locks
            .iter()
            .filter(|(_, lock)| lock.task_id == task_id)
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Check if a specific file is locked.
    pub fn is_locked(&self, file: &PathBuf) -> Option<i64> {
        self.locks.get(file).map(|l| l.task_id)
    }

    /// Get the total number of active locks.
    pub fn lock_count(&self) -> usize {
        self.locks.len()
    }
}

/// Detected language/framework of a task based on file paths and keywords.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDomain {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Ruby,
    Shell,
    Mixed,
    Unknown,
}

/// Agent suitability score for a task.
#[derive(Debug, Clone)]
pub struct AgentMatch {
    pub agent_id: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// Detect the primary domain of a task from its context.
pub fn detect_domain(file_paths: &[String], keywords: &[String]) -> TaskDomain {
    let mut ext_counts: HashMap<&str, usize> = HashMap::new();

    for path in file_paths {
        let ext = path.rsplit('.').next().unwrap_or("");
        match ext {
            "rs" | "toml" => *ext_counts.entry("rust").or_default() += 1,
            "ts" | "tsx" => *ext_counts.entry("typescript").or_default() += 1,
            "js" | "jsx" => *ext_counts.entry("javascript").or_default() += 1,
            "py" => *ext_counts.entry("python").or_default() += 1,
            "go" => *ext_counts.entry("go").or_default() += 1,
            "rb" => *ext_counts.entry("ruby").or_default() += 1,
            "sh" | "bash" => *ext_counts.entry("shell").or_default() += 1,
            _ => {}
        }
    }

    // Also check keywords for language hints
    for kw in keywords {
        match kw.as_str() {
            "rust" | "cargo" | "crate" => *ext_counts.entry("rust").or_default() += 1,
            "typescript" | "react" | "nextjs" | "next" => {
                *ext_counts.entry("typescript").or_default() += 1
            }
            "javascript" | "node" | "npm" => {
                *ext_counts.entry("javascript").or_default() += 1
            }
            "python" | "pip" | "django" | "flask" => {
                *ext_counts.entry("python").or_default() += 1
            }
            "golang" => *ext_counts.entry("go").or_default() += 1,
            _ => {}
        }
    }

    if ext_counts.is_empty() {
        return TaskDomain::Unknown;
    }

    let domains: HashSet<&str> = ext_counts.keys().copied().collect();
    if domains.len() > 1 {
        return TaskDomain::Mixed;
    }

    match ext_counts.keys().next().copied() {
        Some("rust") => TaskDomain::Rust,
        Some("typescript") => TaskDomain::TypeScript,
        Some("javascript") => TaskDomain::JavaScript,
        Some("python") => TaskDomain::Python,
        Some("go") => TaskDomain::Go,
        Some("ruby") => TaskDomain::Ruby,
        Some("shell") => TaskDomain::Shell,
        // tarpaulin-start-ignore
        _ => TaskDomain::Unknown,
        // tarpaulin-stop-ignore
    }
}

/// Score an agent's suitability for a task domain.
///
/// Uses known agent strengths. Agents with MCP support score higher
/// because they can leverage Serena/Sourcegraph context.
pub fn score_agent(
    agent_id: &str,
    domain: &TaskDomain,
    supports_mcp: bool,
    supports_worktree: bool,
) -> AgentMatch {
    let mut score: f64 = 0.5; // Base score
    let mut reasons = Vec::new();

    // Agent-domain affinity
    match (agent_id, domain) {
        ("claude-code", TaskDomain::Rust) => {
            score += 0.3;
            reasons.push("Strong Rust performance".into());
        }
        ("claude-code", TaskDomain::TypeScript) => {
            score += 0.3;
            reasons.push("Strong TypeScript performance".into());
        }
        ("claude-code", _) => {
            score += 0.2;
            reasons.push("General-purpose agent".into());
        }
        ("codex", TaskDomain::Python) => {
            score += 0.3;
            reasons.push("Strong Python performance".into());
        }
        ("codex", _) => {
            score += 0.15;
            reasons.push("Good general performance".into());
        }
        ("aider", _) => {
            score += 0.1;
            reasons.push("Lightweight agent".into());
        }
        ("gemini-cli", _) => {
            score += 0.15;
            reasons.push("Good general performance".into());
        }
        _ => {
            reasons.push("Unknown agent affinity".into());
        }
    }

    // Capability bonuses
    if supports_mcp {
        score += 0.1;
        reasons.push("MCP support (can use Serena/Sourcegraph)".into());
    }
    if supports_worktree {
        score += 0.05;
        reasons.push("Worktree isolation support".into());
    }

    AgentMatch {
        agent_id: agent_id.to_string(),
        score: score.min(1.0),
        reasons,
    }
}

/// Rank agents by suitability for a task.
pub fn rank_agents(
    agents: &[(&str, bool, bool)], // (agent_id, supports_mcp, supports_worktree)
    domain: &TaskDomain,
) -> Vec<AgentMatch> {
    let mut matches: Vec<AgentMatch> = agents
        .iter()
        .map(|(id, mcp, wt)| score_agent(id, domain, *mcp, *wt))
        .collect();
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Lock Manager ──────────────────────────────────────────────

    #[test]
    fn acquire_on_empty_manager() {
        let mut mgr = LockManager::new();
        let result = mgr.try_acquire(1, "claude-code", &[PathBuf::from("src/main.rs")]);
        assert_eq!(result, LockResult::Acquired);
        assert_eq!(mgr.lock_count(), 1);
    }

    #[test]
    fn acquire_multiple_files() {
        let mut mgr = LockManager::new();
        let files = vec![
            PathBuf::from("src/a.rs"),
            PathBuf::from("src/b.rs"),
            PathBuf::from("src/c.rs"),
        ];
        let result = mgr.try_acquire(1, "claude-code", &files);
        assert_eq!(result, LockResult::Acquired);
        assert_eq!(mgr.lock_count(), 3);
    }

    #[test]
    fn conflict_on_locked_file() {
        let mut mgr = LockManager::new();
        mgr.try_acquire(1, "claude-code", &[PathBuf::from("src/main.rs")]);
        let result = mgr.try_acquire(2, "codex", &[PathBuf::from("src/main.rs")]);
        match result {
            LockResult::Conflict(conflicts) => {
                assert_eq!(conflicts.len(), 1);
                assert_eq!(conflicts[0].held_by_task, 1);
                assert_eq!(conflicts[0].held_by_agent, "claude-code");
                assert_eq!(conflicts[0].requested_by_task, 2);
            }
            _ => panic!("Expected conflict"),
        }
    }

    #[test]
    fn same_task_can_reacquire() {
        let mut mgr = LockManager::new();
        mgr.try_acquire(1, "claude-code", &[PathBuf::from("src/main.rs")]);
        let result = mgr.try_acquire(1, "claude-code", &[PathBuf::from("src/main.rs")]);
        assert_eq!(result, LockResult::Acquired);
    }

    #[test]
    fn no_conflict_on_different_files() {
        let mut mgr = LockManager::new();
        mgr.try_acquire(1, "claude-code", &[PathBuf::from("src/auth.rs")]);
        let result = mgr.try_acquire(2, "codex", &[PathBuf::from("src/db.rs")]);
        assert_eq!(result, LockResult::Acquired);
        assert_eq!(mgr.lock_count(), 2);
    }

    #[test]
    fn release_clears_locks() {
        let mut mgr = LockManager::new();
        mgr.try_acquire(
            1,
            "claude-code",
            &[PathBuf::from("a.rs"), PathBuf::from("b.rs")],
        );
        let released = mgr.release(1);
        assert_eq!(released, 2);
        assert_eq!(mgr.lock_count(), 0);
    }

    #[test]
    fn release_only_own_locks() {
        let mut mgr = LockManager::new();
        mgr.try_acquire(1, "claude-code", &[PathBuf::from("a.rs")]);
        mgr.try_acquire(2, "codex", &[PathBuf::from("b.rs")]);
        mgr.release(1);
        assert_eq!(mgr.lock_count(), 1);
        assert!(mgr.is_locked(&PathBuf::from("b.rs")).is_some());
    }

    #[test]
    fn locks_for_task() {
        let mut mgr = LockManager::new();
        mgr.try_acquire(
            1,
            "claude-code",
            &[PathBuf::from("a.rs"), PathBuf::from("b.rs")],
        );
        mgr.try_acquire(2, "codex", &[PathBuf::from("c.rs")]);
        let locks = mgr.locks_for_task(1);
        assert_eq!(locks.len(), 2);
    }

    #[test]
    fn is_locked_returns_task_id() {
        let mut mgr = LockManager::new();
        mgr.try_acquire(1, "claude-code", &[PathBuf::from("a.rs")]);
        assert_eq!(mgr.is_locked(&PathBuf::from("a.rs")), Some(1));
        assert_eq!(mgr.is_locked(&PathBuf::from("b.rs")), None);
    }

    #[test]
    fn partial_conflict_prevents_all_locks() {
        let mut mgr = LockManager::new();
        mgr.try_acquire(1, "claude-code", &[PathBuf::from("shared.rs")]);
        let result = mgr.try_acquire(
            2,
            "codex",
            &[PathBuf::from("unique.rs"), PathBuf::from("shared.rs")],
        );
        match result {
            LockResult::Conflict(_) => {
                // unique.rs should NOT have been locked
                assert!(mgr.is_locked(&PathBuf::from("unique.rs")).is_none());
            }
            _ => panic!("Expected conflict"),
        }
    }

    // ── Domain detection ──────────────────────────────────────────

    #[test]
    fn detect_rust_domain() {
        let domain = detect_domain(&["src/main.rs".into(), "Cargo.toml".into()], &[]);
        assert_eq!(domain, TaskDomain::Rust);
    }

    #[test]
    fn detect_typescript_domain() {
        let domain = detect_domain(&["src/app/page.tsx".into()], &[]);
        assert_eq!(domain, TaskDomain::TypeScript);
    }

    #[test]
    fn detect_mixed_domain() {
        let domain = detect_domain(
            &["src/main.rs".into(), "app/page.tsx".into()],
            &[],
        );
        assert_eq!(domain, TaskDomain::Mixed);
    }

    #[test]
    fn detect_from_keywords() {
        let domain = detect_domain(&[], &["rust".into(), "cargo".into()]);
        assert_eq!(domain, TaskDomain::Rust);
    }

    #[test]
    fn detect_unknown_domain() {
        let domain = detect_domain(&[], &[]);
        assert_eq!(domain, TaskDomain::Unknown);
    }

    // ── Agent scoring ─────────────────────────────────────────────

    #[test]
    fn claude_code_scores_high_for_rust() {
        let m = score_agent("claude-code", &TaskDomain::Rust, true, false);
        assert!(m.score > 0.8);
        assert!(m.reasons.iter().any(|r| r.contains("Rust")));
    }

    #[test]
    fn mcp_support_adds_bonus() {
        let with = score_agent("codex", &TaskDomain::Python, true, false);
        let without = score_agent("codex", &TaskDomain::Python, false, false);
        assert!(with.score > without.score);
    }

    #[test]
    fn rank_agents_sorted_by_score() {
        let agents = vec![
            ("claude-code", true, false),
            ("codex", false, false),
            ("aider", false, false),
        ];
        let ranked = rank_agents(&agents, &TaskDomain::Rust);
        assert_eq!(ranked[0].agent_id, "claude-code");
        for pair in ranked.windows(2) {
            assert!(pair[0].score >= pair[1].score);
        }
    }

    #[test]
    fn score_capped_at_one() {
        let m = score_agent("claude-code", &TaskDomain::Rust, true, true);
        assert!(m.score <= 1.0);
    }

    #[test]
    fn unknown_agent_gets_base_score() {
        let m = score_agent("unknown-agent", &TaskDomain::Rust, false, false);
        assert!((m.score - 0.5).abs() < f64::EPSILON);
    }

    // ── File extension paths ───────────────────────────────────────

    #[test]
    fn detect_python_from_py_extension() {
        let domain = detect_domain(&["app.py".into(), "utils.py".into()], &[]);
        assert_eq!(domain, TaskDomain::Python);
    }

    #[test]
    fn detect_go_from_go_extension() {
        let domain = detect_domain(&["main.go".into(), "handler.go".into()], &[]);
        assert_eq!(domain, TaskDomain::Go);
    }

    #[test]
    fn detect_ruby_from_rb_extension() {
        let domain = detect_domain(&["app.rb".into()], &[]);
        assert_eq!(domain, TaskDomain::Ruby);
    }

    #[test]
    fn detect_shell_from_sh_extension() {
        let domain = detect_domain(&["deploy.sh".into()], &[]);
        assert_eq!(domain, TaskDomain::Shell);
    }

    #[test]
    fn detect_shell_from_bash_extension() {
        let domain = detect_domain(&["install.bash".into()], &[]);
        assert_eq!(domain, TaskDomain::Shell);
    }

    #[test]
    fn detect_javascript_from_js_extension() {
        let domain = detect_domain(&["index.js".into()], &[]);
        assert_eq!(domain, TaskDomain::JavaScript);
    }

    #[test]
    fn detect_javascript_from_jsx_extension() {
        let domain = detect_domain(&["App.jsx".into()], &[]);
        assert_eq!(domain, TaskDomain::JavaScript);
    }

    #[test]
    fn detect_unknown_from_unrecognised_extension() {
        let domain = detect_domain(&["README.md".into(), "image.png".into()], &[]);
        assert_eq!(domain, TaskDomain::Unknown);
    }

    #[test]
    fn detect_mixed_from_files_and_keywords() {
        // py files + rust keyword → Mixed
        let domain = detect_domain(&["app.py".into()], &["rust".into()]);
        assert_eq!(domain, TaskDomain::Mixed);
    }

    // ── Keyword paths ──────────────────────────────────────────────

    #[test]
    fn detect_rust_from_crate_keyword() {
        let domain = detect_domain(&[], &["crate".into()]);
        assert_eq!(domain, TaskDomain::Rust);
    }

    #[test]
    fn detect_typescript_from_typescript_keyword() {
        let domain = detect_domain(&[], &["typescript".into()]);
        assert_eq!(domain, TaskDomain::TypeScript);
    }

    #[test]
    fn detect_typescript_from_react_keyword() {
        let domain = detect_domain(&[], &["react".into()]);
        assert_eq!(domain, TaskDomain::TypeScript);
    }

    #[test]
    fn detect_typescript_from_nextjs_keyword() {
        let domain = detect_domain(&[], &["nextjs".into()]);
        assert_eq!(domain, TaskDomain::TypeScript);
    }

    #[test]
    fn detect_typescript_from_next_keyword() {
        let domain = detect_domain(&[], &["next".into()]);
        assert_eq!(domain, TaskDomain::TypeScript);
    }

    #[test]
    fn detect_javascript_from_javascript_keyword() {
        let domain = detect_domain(&[], &["javascript".into()]);
        assert_eq!(domain, TaskDomain::JavaScript);
    }

    #[test]
    fn detect_javascript_from_node_keyword() {
        let domain = detect_domain(&[], &["node".into()]);
        assert_eq!(domain, TaskDomain::JavaScript);
    }

    #[test]
    fn detect_javascript_from_npm_keyword() {
        let domain = detect_domain(&[], &["npm".into()]);
        assert_eq!(domain, TaskDomain::JavaScript);
    }

    #[test]
    fn detect_python_from_python_keyword() {
        let domain = detect_domain(&[], &["python".into()]);
        assert_eq!(domain, TaskDomain::Python);
    }

    #[test]
    fn detect_python_from_pip_keyword() {
        let domain = detect_domain(&[], &["pip".into()]);
        assert_eq!(domain, TaskDomain::Python);
    }

    #[test]
    fn detect_python_from_django_keyword() {
        let domain = detect_domain(&[], &["django".into()]);
        assert_eq!(domain, TaskDomain::Python);
    }

    #[test]
    fn detect_python_from_flask_keyword() {
        let domain = detect_domain(&[], &["flask".into()]);
        assert_eq!(domain, TaskDomain::Python);
    }

    #[test]
    fn detect_go_from_golang_keyword() {
        let domain = detect_domain(&[], &["golang".into()]);
        assert_eq!(domain, TaskDomain::Go);
    }

    #[test]
    fn detect_unknown_from_unrecognised_keyword() {
        let domain = detect_domain(&[], &["foobar".into()]);
        assert_eq!(domain, TaskDomain::Unknown);
    }

    // ── Agent scoring paths ────────────────────────────────────────

    #[test]
    fn claude_code_scores_high_for_typescript() {
        let m = score_agent("claude-code", &TaskDomain::TypeScript, false, false);
        assert!((m.score - 0.8).abs() < f64::EPSILON);
        assert!(m.reasons.iter().any(|r| r.contains("TypeScript")));
    }

    #[test]
    fn claude_code_general_purpose_for_other_domains() {
        let m = score_agent("claude-code", &TaskDomain::Python, false, false);
        assert!((m.score - 0.7).abs() < f64::EPSILON);
        assert!(m.reasons.iter().any(|r| r.contains("General-purpose")));
    }

    #[test]
    fn codex_scores_high_for_python() {
        let m = score_agent("codex", &TaskDomain::Python, false, false);
        assert!((m.score - 0.8).abs() < f64::EPSILON);
        assert!(m.reasons.iter().any(|r| r.contains("Python")));
    }

    #[test]
    fn codex_general_for_non_python() {
        let m = score_agent("codex", &TaskDomain::Rust, false, false);
        assert!((m.score - 0.65).abs() < f64::EPSILON);
        assert!(m.reasons.iter().any(|r| r.contains("Good general")));
    }

    #[test]
    fn aider_lightweight_for_any_domain() {
        let m = score_agent("aider", &TaskDomain::Go, false, false);
        assert!((m.score - 0.6).abs() < f64::EPSILON);
        assert!(m.reasons.iter().any(|r| r.contains("Lightweight")));
    }

    #[test]
    fn gemini_cli_general_for_any_domain() {
        let m = score_agent("gemini-cli", &TaskDomain::TypeScript, false, false);
        assert!((m.score - 0.65).abs() < f64::EPSILON);
        assert!(m.reasons.iter().any(|r| r.contains("Good general")));
    }

    #[test]
    fn worktree_support_adds_bonus() {
        let with_wt = score_agent("codex", &TaskDomain::Rust, false, true);
        let without_wt = score_agent("codex", &TaskDomain::Rust, false, false);
        assert!(with_wt.score > without_wt.score);
        assert!(with_wt.reasons.iter().any(|r| r.contains("Worktree")));
    }

    #[test]
    fn rank_agents_empty_list() {
        let ranked = rank_agents(&[], &TaskDomain::Rust);
        assert!(ranked.is_empty());
    }

    #[test]
    fn file_conflict_serde_roundtrip() {
        let conflict = FileConflict {
            file_path: PathBuf::from("src/main.rs"),
            held_by_task: 1,
            held_by_agent: "claude-code".into(),
            requested_by_task: 2,
        };
        let json = serde_json::to_string(&conflict).unwrap();
        let parsed: FileConflict = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.file_path, PathBuf::from("src/main.rs"));
        assert_eq!(parsed.held_by_task, 1);
        assert_eq!(parsed.requested_by_task, 2);
    }

    #[test]
    fn task_domain_serde_roundtrip() {
        for domain in [
            TaskDomain::Rust, TaskDomain::TypeScript, TaskDomain::JavaScript,
            TaskDomain::Python, TaskDomain::Go, TaskDomain::Ruby,
            TaskDomain::Shell, TaskDomain::Mixed, TaskDomain::Unknown,
        ] {
            let json = serde_json::to_string(&domain).unwrap();
            let parsed: TaskDomain = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, domain);
        }
    }

    #[test]
    fn lock_manager_release_nonexistent_task() {
        let mut mgr = LockManager::new();
        assert_eq!(mgr.release(999), 0);
    }

    #[test]
    fn locks_for_task_with_no_locks() {
        let mgr = LockManager::new();
        assert!(mgr.locks_for_task(42).is_empty());
    }
}
