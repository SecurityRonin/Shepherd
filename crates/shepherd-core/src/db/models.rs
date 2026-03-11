use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Input,
    Review,
    Error,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Input => "input",
            Self::Review => "review",
            Self::Error => "error",
            Self::Done => "done",
        }
    }

    pub fn parse_status(s: &str) -> Option<Self> {
        match s {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "input" => Some(Self::Input),
            "review" => Some(Self::Review),
            "error" => Some(Self::Error),
            "done" => Some(Self::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub prompt: String,
    pub agent_id: String,
    pub repo_path: String,
    pub branch: String,
    pub isolation_mode: String,
    pub status: TaskStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub prompt: Option<String>,
    pub agent_id: String,
    pub repo_path: Option<String>,
    pub isolation_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: i64,
    pub task_id: i64,
    pub tool_name: String,
    pub tool_args: String,
    pub decision: String,
    pub rule_matched: Option<String>,
    pub decided_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_as_str_all_variants() {
        assert_eq!(TaskStatus::Queued.as_str(), "queued");
        assert_eq!(TaskStatus::Running.as_str(), "running");
        assert_eq!(TaskStatus::Input.as_str(), "input");
        assert_eq!(TaskStatus::Review.as_str(), "review");
        assert_eq!(TaskStatus::Error.as_str(), "error");
        assert_eq!(TaskStatus::Done.as_str(), "done");
    }

    #[test]
    fn test_task_status_parse_status_all_valid() {
        assert_eq!(TaskStatus::parse_status("queued"), Some(TaskStatus::Queued));
        assert_eq!(TaskStatus::parse_status("running"), Some(TaskStatus::Running));
        assert_eq!(TaskStatus::parse_status("input"), Some(TaskStatus::Input));
        assert_eq!(TaskStatus::parse_status("review"), Some(TaskStatus::Review));
        assert_eq!(TaskStatus::parse_status("error"), Some(TaskStatus::Error));
        assert_eq!(TaskStatus::parse_status("done"), Some(TaskStatus::Done));
    }

    #[test]
    fn test_task_status_parse_status_invalid() {
        assert_eq!(TaskStatus::parse_status(""), None);
        assert_eq!(TaskStatus::parse_status("unknown"), None);
        assert_eq!(TaskStatus::parse_status("QUEUED"), None);
        assert_eq!(TaskStatus::parse_status("Running"), None);
    }

    #[test]
    fn test_task_status_serde_roundtrip() {
        let statuses = vec![
            TaskStatus::Queued,
            TaskStatus::Running,
            TaskStatus::Input,
            TaskStatus::Review,
            TaskStatus::Error,
            TaskStatus::Done,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
        // Verify rename_all = snake_case
        let json = serde_json::to_string(&TaskStatus::Queued).unwrap();
        assert_eq!(json, r#""queued""#);
    }

    #[test]
    fn test_create_task_serde_roundtrip() {
        let task = CreateTask {
            title: "Fix bug".into(),
            prompt: Some("Fix the login bug".into()),
            agent_id: "claude-code".into(),
            repo_path: Some("/home/user/repo".into()),
            isolation_mode: Some("worktree".into()),
        };
        let json = serde_json::to_string(&task).unwrap();
        let parsed: CreateTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, "Fix bug");
        assert_eq!(parsed.prompt.as_deref(), Some("Fix the login bug"));
        assert_eq!(parsed.agent_id, "claude-code");
        assert_eq!(parsed.repo_path.as_deref(), Some("/home/user/repo"));
        assert_eq!(parsed.isolation_mode.as_deref(), Some("worktree"));
    }

    #[test]
    fn test_create_task_serde_with_none_fields() {
        let task = CreateTask {
            title: "Minimal".into(),
            prompt: None,
            agent_id: "codex".into(),
            repo_path: None,
            isolation_mode: None,
        };
        let json = serde_json::to_string(&task).unwrap();
        let parsed: CreateTask = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, "Minimal");
        assert!(parsed.prompt.is_none());
        assert!(parsed.repo_path.is_none());
        assert!(parsed.isolation_mode.is_none());
    }

    #[test]
    fn test_permission_serde_roundtrip() {
        let perm = Permission {
            id: 1,
            task_id: 42,
            tool_name: "file_write".into(),
            tool_args: r#"{"path":"/tmp/test"}"#.into(),
            decision: "approved".into(),
            rule_matched: Some("allow_tmp".into()),
            decided_at: Some("2025-01-01T00:00:00".into()),
        };
        let json = serde_json::to_string(&perm).unwrap();
        let parsed: Permission = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.task_id, 42);
        assert_eq!(parsed.tool_name, "file_write");
        assert_eq!(parsed.tool_args, r#"{"path":"/tmp/test"}"#);
        assert_eq!(parsed.decision, "approved");
        assert_eq!(parsed.rule_matched.as_deref(), Some("allow_tmp"));
        assert_eq!(parsed.decided_at.as_deref(), Some("2025-01-01T00:00:00"));
    }

    #[test]
    fn test_permission_serde_with_none_fields() {
        let perm = Permission {
            id: 2,
            task_id: 10,
            tool_name: "bash".into(),
            tool_args: "{}".into(),
            decision: "pending".into(),
            rule_matched: None,
            decided_at: None,
        };
        let json = serde_json::to_string(&perm).unwrap();
        let parsed: Permission = serde_json::from_str(&json).unwrap();
        assert!(parsed.rule_matched.is_none());
        assert!(parsed.decided_at.is_none());
    }
}
