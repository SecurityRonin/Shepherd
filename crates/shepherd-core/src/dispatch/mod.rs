pub mod monitor;

use crate::adapters::AdapterRegistry;
use crate::coordination::LockManager;
use crate::db;
use crate::db::models::{Task, TaskStatus};
use crate::events::{PermissionEvent, ServerEvent, TaskEvent};
use crate::pty::PtyManager;
use crate::yolo::{Decision, YoloEngine};
use monitor::{Detection, SessionMonitor};
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use anyhow::Result;

/// Manages dispatching queued tasks to agent PTY sessions.
pub struct TaskDispatcher {
    db: Arc<Mutex<Connection>>,
    adapters: Arc<AdapterRegistry>,
    pty: Arc<PtyManager>,
    yolo: Arc<YoloEngine>,
    lock_manager: Arc<Mutex<LockManager>>,
    event_tx: broadcast::Sender<ServerEvent>,
    monitors: Arc<Mutex<HashMap<i64, SessionMonitor>>>,
}

impl TaskDispatcher {
    pub fn new(
        db: Arc<Mutex<Connection>>,
        adapters: Arc<AdapterRegistry>,
        pty: Arc<PtyManager>,
        yolo: Arc<YoloEngine>,
        lock_manager: Arc<Mutex<LockManager>>,
        event_tx: broadcast::Sender<ServerEvent>,
    ) -> Self {
        Self {
            db,
            adapters,
            pty,
            yolo,
            lock_manager,
            event_tx,
            monitors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Poll for queued tasks and dispatch them. Called periodically.
    pub async fn poll_and_dispatch(&self) -> Result<Vec<i64>> {
        let queued = {
            let conn = self.db.lock().await;
            db::get_queued_tasks(&conn)?
        };

        let mut dispatched = Vec::new();
        for task in queued {
            match self.dispatch_task(&task).await {
                Ok(()) => dispatched.push(task.id),
                Err(e) => {
                    tracing::error!("Failed to dispatch task {}: {}", task.id, e);
                    let conn = self.db.lock().await;
                    let _ = db::update_task_status(&conn, task.id, TaskStatus::Error);
                }
            }
        }
        Ok(dispatched)
    }

    /// Dispatch a single task to its agent.
    async fn dispatch_task(&self, task: &Task) -> Result<()> {
        // 1. Resolve adapter
        let adapter = self
            .adapters
            .get(&task.agent_id)
            .ok_or_else(|| anyhow::anyhow!("No adapter found for agent: {}", task.agent_id))?;

        // 2. Update status to Dispatching
        {
            let conn = self.db.lock().await;
            db::update_task_status(&conn, task.id, TaskStatus::Dispatching)?;
        }
        let _ = self.event_tx.send(ServerEvent::TaskUpdated(TaskEvent {
            id: task.id,
            title: task.title.clone(),
            agent_id: task.agent_id.clone(),
            status: "dispatching".into(),
            branch: task.branch.clone(),
            repo_path: task.repo_path.clone(),
            iterm2_session_id: task.iterm2_session_id.clone(),
        }));

        // 3. Build command args — append task prompt as final argument
        let mut args = adapter.agent.args.clone();
        let prompt = if task.prompt.is_empty() {
            task.title.clone()
        } else {
            task.prompt.clone()
        };
        args.push(prompt);

        // 4. Spawn PTY session
        self.pty
            .spawn(task.id, &adapter.agent.command, &args, &task.repo_path)
            .await?;

        // 5. Create SessionMonitor for this task
        let monitor = SessionMonitor::new(&adapter.status, &adapter.permissions);
        self.monitors.lock().await.insert(task.id, monitor);

        // 6. Update status to Running
        {
            let conn = self.db.lock().await;
            db::update_task_status(&conn, task.id, TaskStatus::Running)?;
        }
        let _ = self.event_tx.send(ServerEvent::TaskUpdated(TaskEvent {
            id: task.id,
            title: task.title.clone(),
            agent_id: task.agent_id.clone(),
            status: "running".into(),
            branch: task.branch.clone(),
            repo_path: task.repo_path.clone(),
            iterm2_session_id: task.iterm2_session_id.clone(),
        }));

        Ok(())
    }

    /// Handle PTY output for a task — run through SessionMonitor.
    pub async fn handle_pty_output(
        &self,
        task_id: i64,
        output: &str,
    ) -> Result<Option<Detection>> {
        let monitors = self.monitors.lock().await;
        let monitor = match monitors.get(&task_id) {
            Some(m) => m,
            None => return Ok(None),
        };

        let detection = monitor.analyze(output);
        match &detection {
            Detection::PermissionRequest {
                tool_name,
                tool_args,
            } => {
                match self.yolo.evaluate(tool_name, tool_args) {
                    Decision::Allow(_) => {
                        // Auto-approve
                        let seq = monitor.approve_sequence().to_string();
                        drop(monitors);
                        self.pty.write_to(task_id, &seq).await?;
                    }
                    _ => {
                        // Emit permission request event for UI (Ask or Deny)
                        let _ =
                            self.event_tx
                                .send(ServerEvent::PermissionRequested(PermissionEvent {
                                    id: 0,
                                    task_id,
                                    tool_name: tool_name.clone(),
                                    tool_args: tool_args.clone(),
                                    decision: "pending".into(),
                                }));
                        drop(monitors);
                        // Update task status to Input
                        let conn = self.db.lock().await;
                        let _ = db::update_task_status(&conn, task_id, TaskStatus::Input);
                    }
                }
            }
            Detection::Error(_) => {
                drop(monitors);
                let conn = self.db.lock().await;
                let _ = db::update_task_status(&conn, task_id, TaskStatus::Error);
            }
            Detection::Idle => {
                drop(monitors);
                let conn = self.db.lock().await;
                let _ = db::update_task_status(&conn, task_id, TaskStatus::Done);
            }
            _ => {
                drop(monitors);
            }
        }

        Ok(Some(detection))
    }

    /// Approve a pending permission — send approve sequence to PTY.
    pub async fn approve_task(&self, task_id: i64) -> Result<()> {
        let monitors = self.monitors.lock().await;
        let monitor = monitors
            .get(&task_id)
            .ok_or_else(|| anyhow::anyhow!("No active session for task {}", task_id))?;
        let seq = monitor.approve_sequence().to_string();
        drop(monitors);
        self.pty.write_to(task_id, &seq).await?;
        let conn = self.db.lock().await;
        db::update_task_status(&conn, task_id, TaskStatus::Running)?;
        Ok(())
    }

    /// Deny a pending permission — send deny sequence to PTY.
    pub async fn deny_task(&self, task_id: i64) -> Result<()> {
        let monitors = self.monitors.lock().await;
        let monitor = monitors
            .get(&task_id)
            .ok_or_else(|| anyhow::anyhow!("No active session for task {}", task_id))?;
        let seq = monitor.deny_sequence().to_string();
        drop(monitors);
        self.pty.write_to(task_id, &seq).await?;
        let conn = self.db.lock().await;
        db::update_task_status(&conn, task_id, TaskStatus::Running)?;
        Ok(())
    }

    /// Remove monitor for a completed/failed task.
    pub async fn cleanup_task(&self, task_id: i64) {
        self.monitors.lock().await.remove(&task_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::sandbox::SandboxProfile;
    use crate::yolo::rules::RuleSet;

    #[test]
    fn task_dispatcher_constructs() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let _dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);
    }

    #[tokio::test]
    async fn poll_empty_db_returns_empty() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);
        let result = dispatcher.poll_and_dispatch().await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn handle_pty_output_no_monitor_returns_none() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);
        let result = dispatcher.handle_pty_output(999, "some output").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn cleanup_task_removes_monitor() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);

        // Insert a monitor manually
        let status = crate::adapters::protocol::StatusSection {
            working_patterns: vec![],
            idle_patterns: vec![],
            input_patterns: vec![],
            error_patterns: vec![],
        };
        let perms = crate::adapters::protocol::PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(42, SessionMonitor::new(&status, &perms));
        assert_eq!(dispatcher.monitors.lock().await.len(), 1);

        dispatcher.cleanup_task(42).await;
        assert_eq!(dispatcher.monitors.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn approve_task_no_session_errors() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);
        let result = dispatcher.approve_task(999).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No active session"));
    }

    #[tokio::test]
    async fn deny_task_no_session_errors() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);
        let result = dispatcher.deny_task(999).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No active session"));
    }

    #[tokio::test]
    async fn handle_pty_output_with_monitor_detects_working() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);

        let status = crate::adapters::protocol::StatusSection {
            working_patterns: vec![r"Thinking".into()],
            idle_patterns: vec![],
            input_patterns: vec![],
            error_patterns: vec![],
        };
        let perms = crate::adapters::protocol::PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        let result = dispatcher
            .handle_pty_output(1, "Thinking about the problem...")
            .await
            .unwrap();
        assert_eq!(result, Some(Detection::Working));
    }

    #[tokio::test]
    async fn handle_pty_output_detects_idle_and_updates_status() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        // Insert a task so status update works
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp', 'main', 'none', 'running')",
            [],
        ).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, lock_manager, tx);

        let status = crate::adapters::protocol::StatusSection {
            working_patterns: vec![],
            idle_patterns: vec![r"\$\s*$".into()],
            input_patterns: vec![],
            error_patterns: vec![],
        };
        let perms = crate::adapters::protocol::PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        let result = dispatcher.handle_pty_output(1, "$ ").await.unwrap();
        assert_eq!(result, Some(Detection::Idle));

        // Verify status was updated to done
        let conn = db.lock().await;
        let status_str: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status_str, "done");
    }

    #[tokio::test]
    async fn handle_pty_output_detects_error_and_updates_status() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp', 'main', 'none', 'running')",
            [],
        ).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, lock_manager, tx);

        let status = crate::adapters::protocol::StatusSection {
            working_patterns: vec![],
            idle_patterns: vec![],
            input_patterns: vec![],
            error_patterns: vec![r"FATAL".into()],
        };
        let perms = crate::adapters::protocol::PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        let result = dispatcher
            .handle_pty_output(1, "FATAL: something went wrong")
            .await
            .unwrap();
        assert!(matches!(result, Some(Detection::Error(_))));

        // Verify status was updated to error
        let conn = db.lock().await;
        let status_str: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status_str, "error");
    }

    #[tokio::test]
    async fn handle_pty_output_permission_request_emits_event_for_ask() {
        let (tx, mut rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp', 'main', 'none', 'running')",
            [],
        ).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        // Empty rules → Decision::Ask for everything
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, lock_manager, tx);

        let status = crate::adapters::protocol::StatusSection {
            working_patterns: vec![],
            idle_patterns: vec![],
            input_patterns: vec![r"Allow".into()],
            error_patterns: vec![],
        };
        let perms = crate::adapters::protocol::PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        let result = dispatcher
            .handle_pty_output(1, "Allow bash command: cargo test?")
            .await
            .unwrap();
        assert!(matches!(
            result,
            Some(Detection::PermissionRequest { .. })
        ));

        // Should have emitted a PermissionRequested event
        let event = rx.try_recv().unwrap();
        match event {
            ServerEvent::PermissionRequested(p) => {
                assert_eq!(p.task_id, 1);
                assert_eq!(p.decision, "pending");
            }
            other => panic!("Expected PermissionRequested, got {:?}", other),
        }

        // Status should be input
        let conn = db.lock().await;
        let status_str: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status_str, "input");
    }
}
