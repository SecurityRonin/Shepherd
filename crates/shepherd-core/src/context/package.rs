use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// How a context item was discovered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    /// File path directly mentioned in the task description.
    FileReference,
    /// Found via import/dependency analysis of referenced files.
    Structural,
    /// Found via keyword/content matching.
    Semantic,
}

/// A single piece of relevant context discovered for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub source: ContextSource,
    pub file_path: PathBuf,
    /// Relevance score from 0.0 (least) to 1.0 (most).
    pub relevance_score: f64,
    /// Human-readable explanation of why this file is relevant.
    pub reason: String,
}

/// A suggested MCP query for the agent to run at session start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpQuery {
    /// MCP server name (e.g. "serena", "sourcegraph").
    pub server: String,
    /// Tool name (e.g. "find_symbol", "search").
    pub tool: String,
    /// Tool-specific parameters.
    pub params: serde_json::Value,
    /// Why this query would be useful.
    pub reason: String,
}

/// Input to the context orchestrator.
#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub task_id: Option<i64>,
    pub task_title: String,
    pub task_description: String,
    pub repo_path: PathBuf,
    pub agent: String,
    pub max_files: usize,
}

/// Assembled context package ready for injection into an agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackage {
    pub id: String,
    pub task_id: Option<i64>,
    pub items: Vec<ContextItem>,
    pub mcp_queries: Vec<McpQuery>,
    /// Natural-language summary of the context package.
    pub summary: String,
    pub created_at: String,
}

/// Feedback on how effective a context package was.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFeedback {
    pub package_id: String,
    pub task_id: i64,
    pub task_succeeded: bool,
    /// File paths the agent actually opened/used during the session.
    pub items_used: Vec<String>,
    pub agent_duration_secs: Option<f64>,
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_source_serde_roundtrip() {
        let sources = vec![
            ContextSource::FileReference,
            ContextSource::Structural,
            ContextSource::Semantic,
        ];
        for source in sources {
            let json = serde_json::to_string(&source).unwrap();
            let parsed: ContextSource = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, source);
        }
    }

    #[test]
    fn context_item_serde_roundtrip() {
        let item = ContextItem {
            source: ContextSource::Structural,
            file_path: PathBuf::from("src/lib.rs"),
            relevance_score: 0.85,
            reason: "Imported by main.rs".into(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let parsed: ContextItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.source, ContextSource::Structural);
        assert_eq!(parsed.file_path, PathBuf::from("src/lib.rs"));
        assert!((parsed.relevance_score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn mcp_query_serde_roundtrip() {
        let query = McpQuery {
            server: "serena".into(),
            tool: "find_symbol".into(),
            params: serde_json::json!({"name": "UserService"}),
            reason: "Task mentions UserService".into(),
        };
        let json = serde_json::to_string(&query).unwrap();
        let parsed: McpQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.server, "serena");
        assert_eq!(parsed.tool, "find_symbol");
    }

    #[test]
    fn context_package_serde_roundtrip() {
        let pkg = ContextPackage {
            id: "pkg-001".into(),
            task_id: Some(42),
            items: vec![ContextItem {
                source: ContextSource::FileReference,
                file_path: PathBuf::from("src/main.rs"),
                relevance_score: 1.0,
                reason: "Directly mentioned".into(),
            }],
            mcp_queries: vec![],
            summary: "Context for fixing auth bug".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&pkg).unwrap();
        let parsed: ContextPackage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "pkg-001");
        assert_eq!(parsed.task_id, Some(42));
        assert_eq!(parsed.items.len(), 1);
    }

    #[test]
    fn context_feedback_serde_roundtrip() {
        let feedback = ContextFeedback {
            package_id: "pkg-001".into(),
            task_id: 42,
            task_succeeded: true,
            items_used: vec!["src/main.rs".into(), "src/auth.rs".into()],
            agent_duration_secs: Some(120.5),
            notes: Some("Agent completed quickly".into()),
        };
        let json = serde_json::to_string(&feedback).unwrap();
        let parsed: ContextFeedback = serde_json::from_str(&json).unwrap();
        assert!(parsed.task_succeeded);
        assert_eq!(parsed.items_used.len(), 2);
    }
}
