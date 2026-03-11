use serde::{Deserialize, Serialize};

/// Events sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum ServerEvent {
    TaskCreated(TaskEvent),
    TaskUpdated(TaskEvent),
    TaskDeleted { id: i64 },
    TerminalOutput { task_id: i64, data: String },
    PermissionRequested(PermissionEvent),
    PermissionResolved(PermissionEvent),
    GateResult { task_id: i64, gate: String, passed: bool },
    Notification { kind: String, title: String, body: String },
    StatusSnapshot(StatusSnapshot),
}

/// Events sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum ClientEvent {
    TaskCreate {
        title: String,
        agent_id: String,
        repo_path: Option<String>,
        isolation_mode: Option<String>,
        prompt: Option<String>,
    },
    TaskApprove { task_id: i64 },
    TaskApproveAll,
    TaskCancel { task_id: i64 },
    TerminalInput { task_id: i64, data: String },
    TerminalResize { task_id: i64, cols: u16, rows: u16 },
    Subscribe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: i64,
    pub title: String,
    pub agent_id: String,
    pub status: String,
    pub branch: String,
    pub repo_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionEvent {
    pub id: i64,
    pub task_id: i64,
    pub tool_name: String,
    pub tool_args: String,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSnapshot {
    pub tasks: Vec<TaskEvent>,
    pub pending_permissions: Vec<PermissionEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_event_serialization() {
        let event = ServerEvent::TaskCreated(TaskEvent {
            id: 1,
            title: "Test".into(),
            agent_id: "claude-code".into(),
            status: "queued".into(),
            branch: "feat/test".into(),
            repo_path: "/tmp".into(),
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("task_created"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::TaskCreated(t) => assert_eq!(t.id, 1),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_client_event_deserialization() {
        let json = r#"{"type":"task_approve","data":{"task_id":42}}"#;
        let event: ClientEvent = serde_json::from_str(json).unwrap();
        match event {
            ClientEvent::TaskApprove { task_id } => assert_eq!(task_id, 42),
            _ => panic!("wrong variant"),
        }
    }
}
