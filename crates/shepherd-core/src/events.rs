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
    /// Live metrics update for a running task.
    MetricsUpdate(MetricsEvent),
    /// Budget threshold alert (warning or exceeded).
    BudgetAlert(BudgetAlertEvent),
    /// Context package assembled for a task.
    ContextReady(ContextReadyEvent),
    /// Session replay event recorded.
    SessionEvent(SessionLogEvent),
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
    pub iterm2_session_id: Option<String>,
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

/// Live metrics for a running task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsEvent {
    pub task_id: i64,
    pub agent_id: String,
    pub model_id: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub llm_calls: u32,
    pub duration_secs: Option<f64>,
}

/// Budget threshold alert event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlertEvent {
    pub scope: String,
    pub scope_id: String,
    pub status: String,
    pub current_cost: f64,
    pub limit: f64,
    pub percentage: f64,
    pub message: String,
}

/// Context package ready for injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextReadyEvent {
    pub task_id: Option<i64>,
    pub package_id: String,
    pub file_count: usize,
    pub mcp_query_count: usize,
    pub summary: String,
}

/// Structured session log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLogEvent {
    pub task_id: i64,
    pub event_type: String,
    pub content: String,
    pub timestamp: String,
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
            iterm2_session_id: None,
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
                iterm2_session_id: None,
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
    fn test_metrics_update_serialization() {
        let event = ServerEvent::MetricsUpdate(MetricsEvent {
            task_id: 1,
            agent_id: "claude-code".into(),
            model_id: "claude-sonnet-4".into(),
            total_input_tokens: 50_000,
            total_output_tokens: 10_000,
            total_tokens: 60_000,
            total_cost_usd: 0.30,
            llm_calls: 5,
            duration_secs: Some(45.2),
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("metrics_update"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::MetricsUpdate(m) => {
                assert_eq!(m.task_id, 1);
                assert_eq!(m.agent_id, "claude-code");
                assert_eq!(m.total_tokens, 60_000);
                assert!((m.total_cost_usd - 0.30).abs() < f64::EPSILON);
                assert_eq!(m.llm_calls, 5);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_budget_alert_serialization() {
        let event = ServerEvent::BudgetAlert(BudgetAlertEvent {
            scope: "task".into(),
            scope_id: "42".into(),
            status: "exceeded".into(),
            current_cost: 5.50,
            limit: 5.00,
            percentage: 1.10,
            message: "Budget exceeded: $5.50 / $5.00 (110%)".into(),
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("budget_alert"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::BudgetAlert(a) => {
                assert_eq!(a.scope, "task");
                assert_eq!(a.status, "exceeded");
                assert!(a.current_cost > a.limit);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_context_ready_serialization() {
        let event = ServerEvent::ContextReady(ContextReadyEvent {
            task_id: Some(7),
            package_id: "ctx-20260313".into(),
            file_count: 5,
            mcp_query_count: 3,
            summary: "Found 5 relevant files".into(),
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("context_ready"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::ContextReady(c) => {
                assert_eq!(c.task_id, Some(7));
                assert_eq!(c.file_count, 5);
                assert_eq!(c.mcp_query_count, 3);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_session_event_serialization() {
        let event = ServerEvent::SessionEvent(SessionLogEvent {
            task_id: 3,
            event_type: "command".into(),
            content: "cargo test".into(),
            timestamp: "2026-03-13T10:00:00Z".into(),
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("session_event"));
        let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            ServerEvent::SessionEvent(s) => {
                assert_eq!(s.task_id, 3);
                assert_eq!(s.event_type, "command");
                assert_eq!(s.content, "cargo test");
            }
            _ => panic!("wrong variant"),
        }
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
                    iterm2_session_id: None,
                },
                TaskEvent {
                    id: 2,
                    title: "Task B".into(),
                    agent_id: "codex".into(),
                    status: "done".into(),
                    branch: "feat/b".into(),
                    repo_path: "/repo/b".into(),
                    iterm2_session_id: None,
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
