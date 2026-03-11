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

    #[test]
    fn test_server_event_task_deleted_serialization() {
        let event = ServerEvent::TaskDeleted { id: 7 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("task_deleted"));
        assert!(json.contains("7"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::TaskDeleted { id } => assert_eq!(id, 7),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_event_terminal_output_serialization() {
        let event = ServerEvent::TerminalOutput {
            task_id: 3,
            data: "Hello, world!\n".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("terminal_output"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::TerminalOutput { task_id, data } => {
                assert_eq!(task_id, 3);
                assert_eq!(data, "Hello, world!\n");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_event_gate_result_serialization() {
        let event = ServerEvent::GateResult {
            task_id: 5,
            gate: "lint".into(),
            passed: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("gate_result"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::GateResult { task_id, gate, passed } => {
                assert_eq!(task_id, 5);
                assert_eq!(gate, "lint");
                assert!(passed);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_event_notification_serialization() {
        let event = ServerEvent::Notification {
            kind: "warning".into(),
            title: "Disk space low".into(),
            body: "Only 1GB remaining".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("notification"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::Notification { kind, title, body } => {
                assert_eq!(kind, "warning");
                assert_eq!(title, "Disk space low");
                assert_eq!(body, "Only 1GB remaining");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_server_event_status_snapshot_serialization() {
        let snapshot = StatusSnapshot {
            tasks: vec![TaskEvent {
                id: 1,
                title: "Task 1".into(),
                agent_id: "claude-code".into(),
                status: "running".into(),
                branch: "main".into(),
                repo_path: "/repo".into(),
            }],
            pending_permissions: vec![PermissionEvent {
                id: 10,
                task_id: 1,
                tool_name: "bash".into(),
                tool_args: "{}".into(),
                decision: "pending".into(),
            }],
        };
        let event = ServerEvent::StatusSnapshot(snapshot);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("status_snapshot"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::StatusSnapshot(s) => {
                assert_eq!(s.tasks.len(), 1);
                assert_eq!(s.tasks[0].id, 1);
                assert_eq!(s.pending_permissions.len(), 1);
                assert_eq!(s.pending_permissions[0].id, 10);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_client_event_task_create_deserialization() {
        let json = r#"{"type":"task_create","data":{"title":"New task","agent_id":"claude-code","repo_path":"/tmp/repo","isolation_mode":"worktree","prompt":"Do something"}}"#;
        let event: ClientEvent = serde_json::from_str(json).unwrap();
        match event {
            ClientEvent::TaskCreate { title, agent_id, repo_path, isolation_mode, prompt } => {
                assert_eq!(title, "New task");
                assert_eq!(agent_id, "claude-code");
                assert_eq!(repo_path.as_deref(), Some("/tmp/repo"));
                assert_eq!(isolation_mode.as_deref(), Some("worktree"));
                assert_eq!(prompt.as_deref(), Some("Do something"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_client_event_terminal_input_deserialization() {
        let json = r#"{"type":"terminal_input","data":{"task_id":3,"data":"ls -la\n"}}"#;
        let event: ClientEvent = serde_json::from_str(json).unwrap();
        match event {
            ClientEvent::TerminalInput { task_id, data } => {
                assert_eq!(task_id, 3);
                assert_eq!(data, "ls -la\n");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_client_event_terminal_resize_deserialization() {
        let json = r#"{"type":"terminal_resize","data":{"task_id":1,"cols":120,"rows":40}}"#;
        let event: ClientEvent = serde_json::from_str(json).unwrap();
        match event {
            ClientEvent::TerminalResize { task_id, cols, rows } => {
                assert_eq!(task_id, 1);
                assert_eq!(cols, 120);
                assert_eq!(rows, 40);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_client_event_subscribe_deserialization() {
        let json = r#"{"type":"subscribe"}"#;
        let event: ClientEvent = serde_json::from_str(json).unwrap();
        match event {
            ClientEvent::Subscribe => {}
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_permission_event_serde_roundtrip() {
        let perm = PermissionEvent {
            id: 99,
            task_id: 5,
            tool_name: "file_write".into(),
            tool_args: r#"{"path":"/etc/passwd"}"#.into(),
            decision: "denied".into(),
        };
        let json = serde_json::to_string(&perm).unwrap();
        let parsed: PermissionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 99);
        assert_eq!(parsed.task_id, 5);
        assert_eq!(parsed.tool_name, "file_write");
        assert_eq!(parsed.decision, "denied");
    }

    #[test]
    fn test_status_snapshot_serde_roundtrip() {
        let snapshot = StatusSnapshot {
            tasks: vec![
                TaskEvent {
                    id: 1,
                    title: "Task A".into(),
                    agent_id: "claude-code".into(),
                    status: "queued".into(),
                    branch: "feat/a".into(),
                    repo_path: "/repo/a".into(),
                },
                TaskEvent {
                    id: 2,
                    title: "Task B".into(),
                    agent_id: "codex".into(),
                    status: "done".into(),
                    branch: "feat/b".into(),
                    repo_path: "/repo/b".into(),
                },
            ],
            pending_permissions: vec![],
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: StatusSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tasks.len(), 2);
        assert_eq!(parsed.tasks[0].title, "Task A");
        assert_eq!(parsed.tasks[1].title, "Task B");
        assert!(parsed.pending_permissions.is_empty());
    }
}
