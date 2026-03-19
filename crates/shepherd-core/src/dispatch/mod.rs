pub mod monitor;

use crate::adapters::protocol::AdapterConfig;
use crate::adapters::AdapterRegistry;
use crate::context::{
    prepare_injection, ContextOrchestrator, ContextRequest, InjectionPayload, InjectionStrategy,
};
use crate::coordination::LockManager;
use crate::db;
use crate::db::models::{Task, TaskStatus};
use crate::events::{PermissionEvent, ServerEvent, TaskEvent};
use crate::gates::{self, GateConfig};
use crate::observability::{self, BudgetConfig, MetricsAccumulator};
use crate::pty::PtyManager;
use crate::yolo::{Decision, YoloEngine};
use anyhow::Result;
use monitor::{Detection, SessionMonitor};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

/// Build context and prepare injection payload for a task.
///
/// Pure function — no I/O, no async. Returns an InjectionPayload
/// whose `extra_args` should be prepended to the agent command args.
pub fn build_injection_for_task(
    orchestrator: &ContextOrchestrator,
    task: &Task,
    adapter: &AdapterConfig,
) -> InjectionPayload {
    let request = ContextRequest {
        task_id: Some(task.id),
        task_title: task.title.clone(),
        task_description: task.prompt.clone(),
        repo_path: PathBuf::from(&task.repo_path),
        agent: task.agent_id.clone(),
        max_files: 20,
    };

    let package = orchestrator.build_context(&request);
    prepare_injection(
        &package,
        &task.agent_id,
        adapter.capabilities.supports_prompt_arg,
    )
}

/// Manages dispatching queued tasks to agent PTY sessions.
pub struct TaskDispatcher {
    db: Arc<Mutex<Connection>>,
    adapters: Arc<AdapterRegistry>,
    pty: Arc<PtyManager>,
    yolo: Arc<YoloEngine>,
    lock_manager: Arc<Mutex<LockManager>>,
    event_tx: broadcast::Sender<ServerEvent>,
    monitors: Arc<Mutex<HashMap<i64, SessionMonitor>>>,
    metrics_accumulators: Arc<Mutex<HashMap<i64, MetricsAccumulator>>>,
    context: Arc<ContextOrchestrator>,
    gate_config: GateConfig,
    budget_config: BudgetConfig,
    max_agents: usize,
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
            metrics_accumulators: Arc::new(Mutex::new(HashMap::new())),
            context: Arc::new(ContextOrchestrator::new()),
            gate_config: GateConfig::default(),
            budget_config: BudgetConfig::default(),
            max_agents: usize::MAX,
        }
    }

    /// Set the maximum number of concurrent agent sessions.
    pub fn with_max_agents(mut self, max: usize) -> Self {
        self.max_agents = max;
        self
    }

    /// Set the quality gate configuration.
    pub fn with_gate_config(mut self, config: GateConfig) -> Self {
        self.gate_config = config;
        self
    }

    /// Set the budget configuration for cost control.
    pub fn with_budget_config(mut self, config: BudgetConfig) -> Self {
        self.budget_config = config;
        self
    }

    /// Poll for queued tasks and dispatch them. Called periodically.
    /// Respects `max_agents` — stops dispatching when at capacity.
    pub async fn poll_and_dispatch(&self) -> Result<Vec<i64>> {
        let queued = {
            let conn = self.db.lock().await;
            db::get_queued_tasks(&conn)?
        };

        let active_count = self.monitors.lock().await.len();
        let mut remaining_slots = self.max_agents.saturating_sub(active_count);

        let mut dispatched = Vec::new();
        for task in queued {
            if remaining_slots == 0 {
                break;
            }
            match self.dispatch_task(&task).await {
                Ok(true) => {
                    dispatched.push(task.id);
                    remaining_slots -= 1;
                }
                Ok(false) => {
                    // Skipped due to lock conflict — stays queued for next poll
                    tracing::info!(
                        "Task {} skipped: repo {} locked by another task",
                        task.id,
                        task.repo_path
                    );
                }
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
    /// Returns Ok(true) if dispatched, Ok(false) if skipped due to lock conflict.
    async fn dispatch_task(&self, task: &Task) -> Result<bool> {
        // 1. Resolve adapter
        let adapter = self
            .adapters
            .get(&task.agent_id)
            .ok_or_else(|| anyhow::anyhow!("No adapter found for agent: {}", task.agent_id))?;

        // 2. Acquire repo-level lock
        {
            let mut lm = self.lock_manager.lock().await;
            let repo_path = PathBuf::from(&task.repo_path);
            match lm.try_acquire(task.id, &task.agent_id, &[repo_path]) {
                crate::coordination::LockResult::Acquired => {}
                crate::coordination::LockResult::Conflict(_) => return Ok(false),
            }
        }

        // 3. Update status to Dispatching
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

        // 3. Build context and prepare injection
        let injection = build_injection_for_task(&self.context, task, adapter);

        // 4. Build command args — inject context + task prompt
        let mut args = adapter.agent.args.clone();
        // Prepend context extra_args (e.g. -p "Context: ...")
        args.extend(injection.extra_args);
        let prompt = if task.prompt.is_empty() {
            task.title.clone()
        } else {
            task.prompt.clone()
        };
        args.push(prompt);

        // 5. Spawn PTY session
        self.pty
            .spawn(task.id, &adapter.agent.command, &args, &task.repo_path)
            .await?;

        // 5. Create SessionMonitor and MetricsAccumulator for this task
        let monitor = SessionMonitor::new(&adapter.status, &adapter.permissions);
        self.monitors.lock().await.insert(task.id, monitor);
        self.metrics_accumulators
            .lock()
            .await
            .insert(task.id, MetricsAccumulator::new(task.id, &task.agent_id));

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

        Ok(true)
    }

    /// Run quality gates after task completion.
    /// Returns true if all gates passed (or no gates ran), false otherwise.
    async fn run_post_completion_gates(&self, task_id: i64, repo_path: &str) -> bool {
        let results = match gates::run_gates(&PathBuf::from(repo_path), &self.gate_config).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to run gates for task {}: {}", task_id, e);
                return true; // Don't block on gate infrastructure failures
            }
        };

        // Persist each gate result and emit events
        for result in &results {
            // Persist to DB
            {
                let conn = self.db.lock().await;
                let _ = conn.execute(
                    "INSERT INTO gate_results (task_id, gate_name, passed, output) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![task_id, result.gate_name, result.passed as i32, result.output],
                );
            }
            // Emit event
            let _ = self.event_tx.send(ServerEvent::GateResult {
                task_id,
                gate: result.gate_name.clone(),
                passed: result.passed,
            });
        }

        gates::all_gates_passed(&results)
    }

    /// Release file locks held by a task.
    async fn release_locks(&self, task_id: i64) {
        let mut lm = self.lock_manager.lock().await;
        lm.release(task_id);
    }

    /// Finalize metrics for a completed/errored task, persist to DB,
    /// emit MetricsUpdate event, and check budget limits.
    async fn finalize_and_persist_metrics(&self, task_id: i64, status: &str) {
        let accumulator = self.metrics_accumulators.lock().await.remove(&task_id);
        let acc = match accumulator {
            Some(a) => a,
            None => return, // No accumulator — nothing to do
        };

        let metrics = acc.finalize(status);

        // Persist to task_metrics table
        {
            let conn = self.db.lock().await;
            if let Err(e) = observability::store::upsert_metrics(&conn, &metrics) {
                tracing::error!("Failed to persist metrics for task {}: {}", task_id, e);
            }
        }

        // Emit MetricsUpdate event
        let _ = self
            .event_tx
            .send(ServerEvent::MetricsUpdate(crate::events::MetricsEvent {
                task_id: metrics.task_id,
                agent_id: metrics.agent_id.clone(),
                model_id: metrics.model_id.clone(),
                total_input_tokens: metrics.total_input_tokens,
                total_output_tokens: metrics.total_output_tokens,
                total_tokens: metrics.total_tokens,
                total_cost_usd: metrics.total_cost_usd,
                llm_calls: metrics.llm_calls,
                duration_secs: metrics.duration_secs,
            }));

        // Check budgets and emit alerts
        {
            let conn = self.db.lock().await;
            let alerts = observability::check_budgets(
                &self.budget_config,
                &conn,
                metrics.total_cost_usd,
                &task_id.to_string(),
                &metrics.agent_id,
            );
            for alert in alerts {
                let _ =
                    self.event_tx
                        .send(ServerEvent::BudgetAlert(crate::events::BudgetAlertEvent {
                            scope: alert.scope.to_string(),
                            scope_id: alert.scope_id,
                            status: serde_json::to_string(&alert.status)
                                .unwrap_or_else(|_| "unknown".into()),
                            current_cost: alert.current_cost,
                            limit: alert.limit,
                            percentage: alert.percentage,
                            message: alert.message,
                        }));
            }
        }
    }

    /// Handle PTY output for a task — run through SessionMonitor.
    pub async fn handle_pty_output(&self, task_id: i64, output: &str) -> Result<Option<Detection>> {
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
                self.release_locks(task_id).await;
                self.finalize_and_persist_metrics(task_id, "error").await;
                let conn = self.db.lock().await;
                let _ = db::update_task_status(&conn, task_id, TaskStatus::Error);
            }
            Detection::Idle => {
                // Get repo_path before releasing monitors
                let repo_path = {
                    let conn = self.db.lock().await;
                    conn.query_row(
                        "SELECT repo_path FROM tasks WHERE id = ?1",
                        rusqlite::params![task_id],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap_or_default()
                };
                drop(monitors);
                self.release_locks(task_id).await;

                // Run quality gates — if all pass: Done, otherwise: Review
                let all_passed = self.run_post_completion_gates(task_id, &repo_path).await;
                let final_status = if all_passed { "done" } else { "review" };
                self.finalize_and_persist_metrics(task_id, final_status)
                    .await;
                let status = if all_passed {
                    TaskStatus::Done
                } else {
                    TaskStatus::Review
                };
                let conn = self.db.lock().await;
                let _ = db::update_task_status(&conn, task_id, status);
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

    /// Remove monitor, metrics accumulator, and release locks for a completed/failed task.
    pub async fn cleanup_task(&self, task_id: i64) {
        self.monitors.lock().await.remove(&task_id);
        self.metrics_accumulators.lock().await.remove(&task_id);
        self.release_locks(task_id).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::protocol::*;
    use crate::pty::sandbox::SandboxProfile;
    use crate::yolo::rules::RuleSet;

    /// Build an AdapterConfig that uses `echo` as the agent command.
    /// `echo` is safe, exits immediately, and available on all platforms.
    fn echo_adapter() -> crate::adapters::protocol::AdapterConfig {
        AdapterConfig {
            agent: AgentSection {
                name: "echo".to_string(),
                command: "echo".to_string(),
                args: vec!["hello".to_string()],
                args_interactive: vec![],
                version_check: None,
                icon: None,
            },
            hooks: None,
            status: StatusSection {
                working_patterns: vec![],
                idle_patterns: vec![r"\$\s*$".into()],
                input_patterns: vec![r"Allow|Permission".into()],
                error_patterns: vec![r"Error:".into()],
            },
            permissions: PermissionsSection {
                approve: "y\n".into(),
                approve_all: "Y\n".into(),
                deny: "n\n".into(),
                extraction_patterns: vec![],
            },
            capabilities: CapabilitiesSection::default(),
        }
    }

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
        let result = dispatcher
            .handle_pty_output(999, "some output")
            .await
            .unwrap();
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
            extraction_patterns: vec![],
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No active session"));
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No active session"));
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
            extraction_patterns: vec![],
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
            extraction_patterns: vec![],
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
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
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
            extraction_patterns: vec![],
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
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status_str, "error");
    }

    #[tokio::test]
    async fn poll_and_dispatch_with_queued_task_fails_on_missing_adapter() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        // Insert a queued task with an agent_id that has no registered adapter
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test task', 'do something', 'nonexistent-agent', '/tmp/test', 'main', 'none', 'queued')",
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
        // poll_and_dispatch will find the queued task, try to dispatch it, fail on adapter lookup,
        // and set its status to error. The returned vec should be empty (no successful dispatches).
        let result = dispatcher.poll_and_dispatch().await.unwrap();
        assert!(result.is_empty());

        // Verify the task status was set to error
        let conn = db.lock().await;
        let status_str: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
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
            extraction_patterns: vec![],
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
        assert!(matches!(result, Some(Detection::PermissionRequest { .. })));

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
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status_str, "input");
    }

    #[tokio::test]
    async fn handle_pty_output_permission_request_auto_approves_via_yolo() {
        use crate::yolo::rules::Rule;

        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp', 'main', 'none', 'running')",
            [],
        ).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        // Allow rule that matches "bash" tool
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![Rule {
                tool: Some("bash".into()),
                pattern: None,
                path: None,
            }],
        }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);

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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        // "Allow bash command" triggers input_patterns, tool_name extracts "bash",
        // and the yolo rule allows "bash" → auto-approve path
        let result = dispatcher
            .handle_pty_output(1, "Allow bash command: cargo test?")
            .await
            .unwrap();
        assert!(matches!(result, Some(Detection::PermissionRequest { .. })));
    }

    #[tokio::test]
    async fn handle_pty_output_with_monitor_detection_none() {
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

        // Monitor with no patterns → everything returns Detection::None
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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        // This output matches no patterns → Detection::None → falls into the `_ =>` branch
        let result = dispatcher
            .handle_pty_output(1, "just some random output")
            .await
            .unwrap();
        assert_eq!(result, Some(Detection::None));
    }

    #[tokio::test]
    async fn cleanup_task_noop_for_nonexistent() {
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

        // Should not panic or error for a task_id that was never monitored
        dispatcher.cleanup_task(999).await;
        assert_eq!(dispatcher.monitors.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn dispatch_task_with_adapter_spawns_and_sets_running() {
        let (tx, mut rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Test task", "test-echo", "queued", "/tmp", "say hello", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(adapters);
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx);

        let result = dispatcher.poll_and_dispatch().await;
        assert!(result.is_ok());
        let dispatched = result.unwrap();
        // echo should spawn successfully
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0], 1);

        // Check task status is now "running"
        let conn = db.lock().await;
        let status_str: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status_str, "running");

        // Should have emitted TaskUpdated events (dispatching + running)
        let event1 = rx.try_recv().unwrap();
        match event1 {
            ServerEvent::TaskUpdated(t) => assert_eq!(t.status, "dispatching"),
            other => panic!("Expected TaskUpdated(dispatching), got {:?}", other),
        }
        let event2 = rx.try_recv().unwrap();
        match event2 {
            ServerEvent::TaskUpdated(t) => assert_eq!(t.status, "running"),
            other => panic!("Expected TaskUpdated(running), got {:?}", other),
        }

        // A monitor should have been registered for the task
        assert!(dispatcher.monitors.lock().await.contains_key(&1));
    }

    #[tokio::test]
    async fn dispatch_task_uses_title_when_prompt_empty() {
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        // Empty prompt — should use title as the argument
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Title as prompt", "test-echo", "queued", "/tmp", "", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(adapters);
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx);

        let result = dispatcher.poll_and_dispatch().await.unwrap();
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn approve_task_with_monitor_succeeds_no_pty_handle() {
        // Test approve_task when a monitor exists but no PTY handle is registered.
        // write_to returns Ok(()) when no handle is found (no-op), so the full
        // code path through update_task_status executes.
        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp', 'main', 'none', 'input')",
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

        // Manually insert a monitor for task 1 (no PTY handle exists)
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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        // approve_task should succeed: monitor found, write_to is no-op, status updated
        let result = dispatcher.approve_task(1).await;
        assert!(result.is_ok());

        // Verify status was updated to running
        let conn = db.lock().await;
        let status_str: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status_str, "running");
    }

    #[tokio::test]
    async fn deny_task_with_monitor_succeeds_no_pty_handle() {
        // Same pattern: monitor exists but no PTY handle.
        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp', 'main', 'none', 'input')",
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
            error_patterns: vec![],
        };
        let perms = crate::adapters::protocol::PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        let result = dispatcher.deny_task(1).await;
        assert!(result.is_ok());

        // Verify status was updated to running
        let conn = db.lock().await;
        let status_str: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status_str, "running");
    }

    #[tokio::test]
    async fn approve_task_with_real_dispatch() {
        // Also test through the dispatch path to exercise the full flow
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Test", "test-echo", "queued", "/tmp", "hello", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(adapters);
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty.clone(), yolo, locks, tx);

        // Dispatch first to register a monitor
        let dispatched = dispatcher.poll_and_dispatch().await.unwrap();
        assert_eq!(dispatched.len(), 1);

        // Give the echo process a moment
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Try to approve — may succeed or fail depending on whether echo already exited
        let _ = dispatcher.approve_task(1).await;
    }

    #[tokio::test]
    async fn deny_task_with_real_dispatch() {
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Test", "test-echo", "queued", "/tmp", "hello", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(adapters);
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty.clone(), yolo, locks, tx);

        let dispatched = dispatcher.poll_and_dispatch().await.unwrap();
        assert_eq!(dispatched.len(), 1);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let _ = dispatcher.deny_task(1).await;
    }

    #[tokio::test]
    async fn dispatch_multiple_queued_tasks_different_repos() {
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        // Two tasks in DIFFERENT repos — both should dispatch
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Task 1", "test-echo", "queued", "/tmp/repo-a", "first", "main", "none"],
        ).unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Task 2", "test-echo", "queued", "/tmp/repo-b", "second", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(adapters);
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, locks, tx);

        let dispatched = dispatcher.poll_and_dispatch().await.unwrap();
        assert_eq!(dispatched.len(), 2);
    }

    #[tokio::test]
    async fn dispatch_skips_conflicting_same_repo_task() {
        // Two queued tasks in the SAME repo — only the first should dispatch,
        // the second should be skipped (left queued) due to repo lock conflict.
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Task 1", "test-echo", "queued", "/tmp/same-repo", "first", "main", "none"],
        ).unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Task 2", "test-echo", "queued", "/tmp/same-repo", "second", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(adapters);
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx);

        let dispatched = dispatcher.poll_and_dispatch().await.unwrap();
        // Only first task dispatched; second stayed queued due to repo lock
        assert_eq!(dispatched.len(), 1);
        assert_eq!(dispatched[0], 1);

        // Second task should still be queued
        let conn = db.lock().await;
        let status: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 2", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status, "queued");
    }

    #[tokio::test]
    async fn dispatch_acquires_and_cleanup_releases_lock() {
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Task 1", "test-echo", "queued", "/tmp/locked-repo", "hello", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(adapters);
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks.clone(), tx);

        let dispatched = dispatcher.poll_and_dispatch().await.unwrap();
        assert_eq!(dispatched.len(), 1);

        // Lock should be held
        {
            let lm = locks.lock().await;
            assert!(lm
                .is_locked(&std::path::PathBuf::from("/tmp/locked-repo"))
                .is_some());
        }

        // Cleanup releases the lock
        dispatcher.cleanup_task(1).await;

        {
            let lm = locks.lock().await;
            assert!(lm
                .is_locked(&std::path::PathBuf::from("/tmp/locked-repo"))
                .is_none());
        }
    }

    #[tokio::test]
    async fn error_detection_releases_lock() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp/err-repo', 'main', 'none', 'running')",
            [],
        ).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        // Pre-acquire lock as if dispatch_task had run
        {
            let mut lm = locks.lock().await;
            lm.try_acquire(
                1,
                "claude-code",
                &[std::path::PathBuf::from("/tmp/err-repo")],
            );
        }

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks.clone(), tx);

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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        // Trigger error detection
        let result = dispatcher
            .handle_pty_output(1, "FATAL: crash")
            .await
            .unwrap();
        assert!(matches!(result, Some(Detection::Error(_))));

        // Lock should be released
        let lm = locks.lock().await;
        assert!(lm
            .is_locked(&std::path::PathBuf::from("/tmp/err-repo"))
            .is_none());
    }

    #[tokio::test]
    async fn idle_detection_releases_lock() {
        let (tx, _rx) = broadcast::channel(16);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', '/tmp/done-repo', 'main', 'none', 'running')",
            [],
        ).unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        // Pre-acquire lock
        {
            let mut lm = locks.lock().await;
            lm.try_acquire(
                1,
                "claude-code",
                &[std::path::PathBuf::from("/tmp/done-repo")],
            );
        }

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks.clone(), tx);

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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        let result = dispatcher.handle_pty_output(1, "$ ").await.unwrap();
        assert_eq!(result, Some(Detection::Idle));

        // Lock should be released
        let lm = locks.lock().await;
        assert!(lm
            .is_locked(&std::path::PathBuf::from("/tmp/done-repo"))
            .is_none());
    }

    // ── Concurrency limiting tests ───────────────────────────────

    #[tokio::test]
    async fn max_agents_limits_concurrent_dispatches() {
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        // Insert 3 queued tasks in different repos (no lock conflicts)
        for i in 1..=3 {
            conn.execute(
                "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![format!("Task {i}"), "test-echo", "queued", format!("/tmp/repo-{i}"), "hello", "main", "none"],
            ).unwrap();
        }

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), Arc::new(adapters), pty, yolo, locks, tx)
            .with_max_agents(2);

        let dispatched = dispatcher.poll_and_dispatch().await.unwrap();
        // Only 2 should dispatch despite 3 being queued
        assert_eq!(dispatched.len(), 2);

        // Third task should still be queued
        let conn = db.lock().await;
        let status: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 3", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status, "queued");
    }

    #[tokio::test]
    async fn max_agents_accounts_for_already_active_sessions() {
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["New task", "test-echo", "queued", "/tmp/repo-new", "hello", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), Arc::new(adapters), pty, yolo, locks, tx)
            .with_max_agents(1);

        // Simulate an already-active session by inserting a monitor
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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(99, SessionMonitor::new(&status, &perms));

        // max_agents=1, 1 already active → 0 should dispatch
        let dispatched = dispatcher.poll_and_dispatch().await.unwrap();
        assert_eq!(dispatched.len(), 0);

        // Task should still be queued
        let conn = db.lock().await;
        let status_str: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status_str, "queued");
    }

    #[tokio::test]
    async fn max_agents_zero_dispatches_nothing() {
        let (tx, _rx) = broadcast::channel(256);
        let conn = crate::db::open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, agent_id, status, repo_path, prompt, branch, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Task", "test-echo", "queued", "/tmp/repo", "hello", "main", "none"],
        ).unwrap();

        let mut adapters = AdapterRegistry::new();
        adapters.register("test-echo".into(), echo_adapter());

        let db = Arc::new(Mutex::new(conn));
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher =
            TaskDispatcher::new(db, Arc::new(adapters), pty, yolo, locks, tx).with_max_agents(0);

        let dispatched = dispatcher.poll_and_dispatch().await.unwrap();
        assert_eq!(dispatched.len(), 0);
    }

    // ── Context injection tests ──────────────────────────────────

    #[test]
    fn build_injection_claude_code_uses_claude_md_strategy() {
        let orchestrator = ContextOrchestrator::new();
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();

        let task = Task {
            id: 1,
            title: "Fix auth bug".into(),
            prompt: "The bug is in src/main.rs".into(),
            agent_id: "claude-code".into(),
            repo_path: tmp.path().to_string_lossy().to_string(),
            branch: "main".into(),
            isolation_mode: "none".into(),
            status: TaskStatus::Queued,
            created_at: String::new(),
            updated_at: String::new(),
            iterm2_session_id: None,
        };

        let adapter = echo_adapter();
        let payload = build_injection_for_task(&orchestrator, &task, &adapter);

        assert_eq!(payload.strategy, InjectionStrategy::ClaudeMd);
        // ClaudeMd strategy: content is full markdown, extra_args is empty
        assert!(payload.content.contains("Shepherd Context"));
        assert!(payload.extra_args.is_empty());
    }

    #[test]
    fn build_injection_prompt_arg_agent_gets_extra_args() {
        let orchestrator = ContextOrchestrator::new();
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/app.py"), "def main(): pass").unwrap();

        let task = Task {
            id: 2,
            title: "Fix Python bug".into(),
            prompt: "Check src/app.py".into(),
            agent_id: "codex".into(),
            repo_path: tmp.path().to_string_lossy().to_string(),
            branch: "main".into(),
            isolation_mode: "none".into(),
            status: TaskStatus::Queued,
            created_at: String::new(),
            updated_at: String::new(),
            iterm2_session_id: None,
        };

        // Adapter with supports_prompt_arg = true
        let mut adapter = echo_adapter();
        adapter.capabilities.supports_prompt_arg = true;

        let payload = build_injection_for_task(&orchestrator, &task, &adapter);

        assert_eq!(payload.strategy, InjectionStrategy::PromptArg);
        assert!(!payload.extra_args.is_empty());
        assert_eq!(payload.extra_args[0], "-p");
    }

    #[test]
    fn build_injection_stdin_agent_no_extra_args() {
        let orchestrator = ContextOrchestrator::new();
        let tmp = tempfile::tempdir().unwrap();

        let task = Task {
            id: 3,
            title: "Do something".into(),
            prompt: "".into(),
            agent_id: "opencode".into(),
            repo_path: tmp.path().to_string_lossy().to_string(),
            branch: "main".into(),
            isolation_mode: "none".into(),
            status: TaskStatus::Queued,
            created_at: String::new(),
            updated_at: String::new(),
            iterm2_session_id: None,
        };

        let mut adapter = echo_adapter();
        adapter.capabilities.supports_prompt_arg = false;

        let payload = build_injection_for_task(&orchestrator, &task, &adapter);

        assert_eq!(payload.strategy, InjectionStrategy::StdinMessage);
        assert!(payload.extra_args.is_empty());
        // Content should still be generated
        assert!(payload.content.contains("Context:"));
    }

    #[test]
    fn build_injection_empty_repo_no_crash() {
        let orchestrator = ContextOrchestrator::new();
        let tmp = tempfile::tempdir().unwrap();

        let task = Task {
            id: 4,
            title: "New task".into(),
            prompt: "".into(),
            agent_id: "claude-code".into(),
            repo_path: tmp.path().to_string_lossy().to_string(),
            branch: "main".into(),
            isolation_mode: "none".into(),
            status: TaskStatus::Queued,
            created_at: String::new(),
            updated_at: String::new(),
            iterm2_session_id: None,
        };

        let adapter = echo_adapter();
        let payload = build_injection_for_task(&orchestrator, &task, &adapter);

        // Should produce a valid payload even with empty repo
        assert_eq!(payload.strategy, InjectionStrategy::ClaudeMd);
        assert!(payload.content.contains("Shepherd Context"));
    }

    // ── Quality gates tests ──────────────────────────────────────

    #[tokio::test]
    async fn idle_runs_gates_and_sets_done_when_all_pass() {
        let (tx, mut rx) = broadcast::channel(64);
        let conn = crate::db::open_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();

        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', ?1, 'main', 'none', 'running')",
            rusqlite::params![repo_path],
        ).unwrap();

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        // Disable all gates — empty project → no gates run → all_gates_passed = true
        let gate_config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: false,
            custom_gates: vec![],
            timeout_seconds: 5,
        };

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx)
            .with_gate_config(gate_config);

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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        let result = dispatcher.handle_pty_output(1, "$ ").await.unwrap();
        assert_eq!(result, Some(Detection::Idle));

        // Status should be Done (all gates passed / no gates ran)
        let conn = db.lock().await;
        let task_status: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(task_status, "done");
    }

    #[tokio::test]
    async fn idle_runs_gates_and_sets_review_when_gate_fails() {
        let (tx, _rx) = broadcast::channel(64);
        let conn = crate::db::open_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();

        // Create a Cargo.toml so project is detected as Rust, triggering lint gate
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"fake\"").unwrap();

        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', ?1, 'main', 'none', 'running')",
            rusqlite::params![repo_path],
        ).unwrap();

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        // Enable only test gate — will fail on a fake Cargo.toml project
        let gate_config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: true,
            custom_gates: vec![],
            timeout_seconds: 10,
        };

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx)
            .with_gate_config(gate_config);

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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        let result = dispatcher.handle_pty_output(1, "$ ").await.unwrap();
        assert_eq!(result, Some(Detection::Idle));

        // Status should be Review (test gate failed on fake project)
        let conn = db.lock().await;
        let task_status: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(task_status, "review");

        // Gate results should be persisted
        let gate_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM gate_results WHERE task_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(gate_count > 0);
    }

    #[tokio::test]
    async fn gate_results_emit_events() {
        let (tx, mut rx) = broadcast::channel(64);
        let conn = crate::db::open_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();

        // Create a custom gate script that passes
        let gate_script = tmp.path().join("gate.sh");
        std::fs::write(&gate_script, "#!/bin/sh\necho ok\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&gate_script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', ?1, 'main', 'none', 'running')",
            rusqlite::params![repo_path],
        ).unwrap();

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let gate_config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: false,
            custom_gates: vec![gate_script.to_string_lossy().to_string()],
            timeout_seconds: 10,
        };

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx)
            .with_gate_config(gate_config);

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
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        dispatcher.handle_pty_output(1, "$ ").await.unwrap();

        // Should have received a GateResult event
        let mut found_gate_event = false;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, ServerEvent::GateResult { .. }) {
                found_gate_event = true;
                break;
            }
        }
        assert!(found_gate_event, "Should have emitted a GateResult event");
    }

    // ── Observability metrics tests ──────────────────────────────────

    #[tokio::test]
    async fn idle_finalizes_and_persists_metrics() {
        use crate::observability;

        let (tx, _rx) = broadcast::channel(64);
        let conn = crate::db::open_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();

        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', ?1, 'main', 'none', 'running')",
            rusqlite::params![repo_path],
        ).unwrap();

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let gate_config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: false,
            custom_gates: vec![],
            timeout_seconds: 5,
        };

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx)
            .with_gate_config(gate_config);

        // Insert a monitor
        let status = StatusSection {
            working_patterns: vec![],
            idle_patterns: vec![r"\$\s*$".into()],
            input_patterns: vec![],
            error_patterns: vec![],
        };
        let perms = PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        // Insert a metrics accumulator for the task
        dispatcher
            .metrics_accumulators
            .lock()
            .await
            .insert(1, observability::MetricsAccumulator::new(1, "claude-code"));

        // Trigger idle detection
        let result = dispatcher.handle_pty_output(1, "$ ").await.unwrap();
        assert_eq!(result, Some(Detection::Idle));

        // Verify metrics were persisted to task_metrics table
        let conn = db.lock().await;
        let metrics = observability::store::get_task_metrics(&conn, 1).unwrap();
        assert!(metrics.is_some(), "Metrics should be persisted on idle");
        let m = metrics.unwrap();
        assert_eq!(m.agent_id, "claude-code");
        assert_eq!(m.status, "done");
        assert!(m.duration_secs.is_some());
    }

    #[tokio::test]
    async fn error_finalizes_and_persists_metrics() {
        use crate::observability;

        let (tx, _rx) = broadcast::channel(64);
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
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx);

        // Insert a monitor that detects errors
        let status = StatusSection {
            working_patterns: vec![],
            idle_patterns: vec![],
            input_patterns: vec![],
            error_patterns: vec![r"Error:".into()],
        };
        let perms = PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));

        // Insert a metrics accumulator
        dispatcher
            .metrics_accumulators
            .lock()
            .await
            .insert(1, observability::MetricsAccumulator::new(1, "claude-code"));

        // Trigger error detection
        let result = dispatcher
            .handle_pty_output(1, "Error: something broke")
            .await
            .unwrap();
        assert!(matches!(result, Some(Detection::Error(_))));

        // Verify metrics were persisted with error status
        let conn = db.lock().await;
        let metrics = observability::store::get_task_metrics(&conn, 1).unwrap();
        assert!(metrics.is_some(), "Metrics should be persisted on error");
        let m = metrics.unwrap();
        assert_eq!(m.status, "error");
    }

    #[tokio::test]
    async fn idle_emits_metrics_update_event() {
        use crate::observability;

        let (tx, mut rx) = broadcast::channel(64);
        let conn = crate::db::open_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();

        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', ?1, 'main', 'none', 'running')",
            rusqlite::params![repo_path],
        ).unwrap();

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let gate_config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: false,
            custom_gates: vec![],
            timeout_seconds: 5,
        };

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx)
            .with_gate_config(gate_config);

        let status = StatusSection {
            working_patterns: vec![],
            idle_patterns: vec![r"\$\s*$".into()],
            input_patterns: vec![],
            error_patterns: vec![],
        };
        let perms = PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));
        dispatcher
            .metrics_accumulators
            .lock()
            .await
            .insert(1, observability::MetricsAccumulator::new(1, "claude-code"));

        dispatcher.handle_pty_output(1, "$ ").await.unwrap();

        // Check for MetricsUpdate event
        let mut found_metrics_event = false;
        while let Ok(event) = rx.try_recv() {
            if let ServerEvent::MetricsUpdate(m) = event {
                assert_eq!(m.task_id, 1);
                assert_eq!(m.agent_id, "claude-code");
                found_metrics_event = true;
                break;
            }
        }
        assert!(
            found_metrics_event,
            "Should have emitted a MetricsUpdate event"
        );
    }

    #[tokio::test]
    async fn idle_checks_budgets_and_emits_alerts() {
        use crate::observability;
        use crate::observability::BudgetConfig;

        let (tx, mut rx) = broadcast::channel(64);
        let conn = crate::db::open_memory().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_string_lossy().to_string();

        conn.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Test', '', 'claude-code', ?1, 'main', 'none', 'running')",
            rusqlite::params![repo_path],
        ).unwrap();

        // Pre-insert some high cost metrics so budget is already high
        let mut acc = observability::MetricsAccumulator::new(99, "claude-code");
        acc.record("claude-sonnet-4", 500_000, 100_000);
        observability::store::upsert_metrics(&conn, &acc.finalize("done")).unwrap();

        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let gate_config = GateConfig {
            lint: false,
            format_check: false,
            type_check: false,
            test: false,
            custom_gates: vec![],
            timeout_seconds: 5,
        };

        // Set a very tight daily budget to trigger an alert
        let budget_config = BudgetConfig {
            max_cost_per_task: None,
            max_cost_per_agent_daily: Some(0.01),
            max_cost_daily: None,
            warning_threshold: 0.8,
        };

        let dispatcher = TaskDispatcher::new(db.clone(), adapters, pty, yolo, locks, tx)
            .with_gate_config(gate_config)
            .with_budget_config(budget_config);

        let status = StatusSection {
            working_patterns: vec![],
            idle_patterns: vec![r"\$\s*$".into()],
            input_patterns: vec![],
            error_patterns: vec![],
        };
        let perms = PermissionsSection {
            approve: "y\n".into(),
            approve_all: "Y\n".into(),
            deny: "n\n".into(),
            extraction_patterns: vec![],
        };
        dispatcher
            .monitors
            .lock()
            .await
            .insert(1, SessionMonitor::new(&status, &perms));
        dispatcher
            .metrics_accumulators
            .lock()
            .await
            .insert(1, observability::MetricsAccumulator::new(1, "claude-code"));

        dispatcher.handle_pty_output(1, "$ ").await.unwrap();

        // Check for BudgetAlert event
        let mut found_budget_alert = false;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, ServerEvent::BudgetAlert(_)) {
                found_budget_alert = true;
                break;
            }
        }
        assert!(
            found_budget_alert,
            "Should have emitted a BudgetAlert event"
        );
    }

    #[tokio::test]
    async fn cleanup_removes_metrics_accumulator() {
        use crate::observability;

        let (tx, _rx) = broadcast::channel(16);
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(RuleSet {
            deny: vec![],
            allow: vec![],
        }));
        let locks = Arc::new(Mutex::new(LockManager::new()));

        let dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, locks, tx);

        // Insert both a monitor and metrics accumulator
        dispatcher.monitors.lock().await.insert(
            1,
            SessionMonitor::new(
                &StatusSection {
                    working_patterns: vec![],
                    idle_patterns: vec![],
                    input_patterns: vec![],
                    error_patterns: vec![],
                },
                &PermissionsSection {
                    approve: "y\n".into(),
                    approve_all: "Y\n".into(),
                    deny: "n\n".into(),
                    extraction_patterns: vec![],
                },
            ),
        );
        dispatcher
            .metrics_accumulators
            .lock()
            .await
            .insert(1, observability::MetricsAccumulator::new(1, "test"));

        assert!(dispatcher
            .metrics_accumulators
            .lock()
            .await
            .contains_key(&1));

        dispatcher.cleanup_task(1).await;

        assert!(!dispatcher.monitors.lock().await.contains_key(&1));
        assert!(!dispatcher
            .metrics_accumulators
            .lock()
            .await
            .contains_key(&1));
    }
}
