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
