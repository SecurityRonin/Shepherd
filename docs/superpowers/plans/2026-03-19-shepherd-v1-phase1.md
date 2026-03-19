# Shepherd v1.0 Phase 1: Core Loop Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the end-to-end task dispatch loop so a user can launch the Tauri app, create a task, and watch Claude Code execute it in a live terminal — with permission approval/YOLO support.

**Architecture:** Embed the Axum server in-process inside Tauri (replacing the current child process spawn). Add a `TaskDispatcher` background loop that watches for `Queued` tasks, resolves adapters, spawns PTY sessions, and streams output over WebSocket. Wire the React frontend's Zustand store to real Tauri commands instead of mock data.

**Tech Stack:** Rust (Tauri 2, Axum, tokio, portable-pty), TypeScript (React 18, Zustand 5, xterm.js 5), SQLite

**Spec:** `docs/superpowers/specs/2026-03-19-shepherd-v1-design.md`

---

## File Map

### New Files
- `crates/shepherd-core/src/dispatch/mod.rs` — TaskDispatcher: background loop, adapter resolution, PTY orchestration
- `crates/shepherd-core/src/dispatch/monitor.rs` — SessionMonitor: parse PTY output for permission patterns, emit events
- `crates/shepherd-server/src/startup.rs` — `start_server()` reusable function + server.json discovery/lockfile
- `src/lib/tauri.ts` — Tauri invoke wrappers (typed bridge between React and Rust commands)

### Modified Files
- `crates/shepherd-core/src/lib.rs` — add `pub mod dispatch;`
- `crates/shepherd-core/src/db/models.rs` — add `Dispatching` variant to `TaskStatus`
- `crates/shepherd-core/src/db/mod.rs` — add `update_task_status()` and `get_queued_tasks()` DB functions
- `crates/shepherd-core/src/events.rs` — add `TaskDispatching` server event variant
- `crates/shepherd-server/src/lib.rs` — add `pub mod startup;`, export `start_server()`
- `crates/shepherd-server/src/state.rs` — add `LockManager` and `TaskDispatcher` handle to `AppState`
- `crates/shepherd-server/src/ws.rs` — handle `TaskApprove` → route to SessionMonitor
- `src-tauri/src/lib.rs` — replace child process spawn with in-process `start_server()`, add Tauri commands
- `src-tauri/Cargo.toml` — add `shepherd-server` dependency
- `src/lib/api.ts` — add Tauri invoke path alongside HTTP fallback
- `src/store/tasks.ts` — wire to Tauri commands / real API
- `src/store/sessions.ts` — wire terminal data from WebSocket
- `src/hooks/useWebSocket.ts` — connect to embedded server port
- `src/features/focus/Terminal.tsx` — wire xterm.js to live PTY stream
- `src/features/focus/PermissionPrompt.tsx` — wire approve/deny to real events

---

## Task 1: Add `Dispatching` Status to TaskStatus Enum

**Files:**
- Modify: `crates/shepherd-core/src/db/models.rs:5-36`
- Test: existing tests in `crates/shepherd-core/src/db/models.rs:76-202`

- [ ] **Step 1: Write the failing test**

Add to `crates/shepherd-core/src/db/models.rs` in the `tests` module:

```rust
#[test]
fn test_task_status_dispatching_variant() {
    assert_eq!(TaskStatus::Dispatching.as_str(), "dispatching");
    assert_eq!(TaskStatus::parse_status("dispatching"), Some(TaskStatus::Dispatching));
    let json = serde_json::to_string(&TaskStatus::Dispatching).unwrap();
    assert_eq!(json, r#""dispatching""#);
    let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, TaskStatus::Dispatching);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-core db::models::tests::test_task_status_dispatching_variant -- --nocapture`
Expected: FAIL — `Dispatching` variant does not exist

- [ ] **Step 3: Add `Dispatching` to `TaskStatus`**

In `crates/shepherd-core/src/db/models.rs`, add the variant to the enum:

```rust
pub enum TaskStatus {
    Queued,
    Dispatching,  // NEW
    Running,
    Input,
    Review,
    Error,
    Done,
}
```

Add to `as_str()`:
```rust
Self::Dispatching => "dispatching",
```

Add to `parse_status()`:
```rust
"dispatching" => Some(Self::Dispatching),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p shepherd-core db::models::tests -- --nocapture`
Expected: ALL PASS (including existing tests)

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/db/models.rs
git commit -m "feat: add Dispatching variant to TaskStatus enum"
```

---

## Task 2: Add DB Helper Functions for Dispatch

**Files:**
- Modify: `crates/shepherd-core/src/db/mod.rs`
- Test: `crates/shepherd-core/src/db/mod.rs` (in existing test module)

- [ ] **Step 1: Write failing tests**

Add to the test module in `crates/shepherd-core/src/db/mod.rs`:

```rust
#[test]
fn test_update_task_status() {
    let conn = open_memory().unwrap();
    // Insert a task
    conn.execute(
        "INSERT INTO tasks (title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["Test", "Do thing", "claude-code", "/tmp", "main", "worktree", "queued"],
    ).unwrap();

    update_task_status(&conn, 1, TaskStatus::Dispatching).unwrap();
    let tasks = list_tasks(&conn).unwrap();
    assert_eq!(tasks[0].status, TaskStatus::Dispatching);
}

#[test]
fn test_get_queued_tasks() {
    let conn = open_memory().unwrap();
    conn.execute(
        "INSERT INTO tasks (title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["Queued", "", "claude-code", "/tmp", "main", "worktree", "queued"],
    ).unwrap();
    conn.execute(
        "INSERT INTO tasks (title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["Running", "", "aider", "/tmp", "main", "worktree", "running"],
    ).unwrap();

    let queued = get_queued_tasks(&conn).unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].title, "Queued");
    assert_eq!(queued[0].status, TaskStatus::Queued);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p shepherd-core db::tests::test_update_task_status -- --nocapture`
Expected: FAIL — functions not defined

- [ ] **Step 3: Implement `update_task_status` and `get_queued_tasks`**

Add to `crates/shepherd-core/src/db/mod.rs`:

```rust
use crate::db::models::TaskStatus;

pub fn update_task_status(conn: &Connection, task_id: i64, status: TaskStatus) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![status.as_str(), task_id],
    )?;
    Ok(())
}

pub fn get_queued_tasks(conn: &Connection) -> Result<Vec<models::Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, prompt, agent_id, repo_path, branch, isolation_mode, status, created_at, updated_at, iterm2_session_id FROM tasks WHERE status = 'queued' ORDER BY created_at ASC"
    )?;
    let tasks = stmt.query_map([], |row| {
        Ok(models::Task {
            id: row.get(0)?,
            title: row.get(1)?,
            prompt: row.get(2)?,
            agent_id: row.get(3)?,
            repo_path: row.get(4)?,
            branch: row.get(5)?,
            isolation_mode: row.get(6)?,
            status: TaskStatus::parse_status(&row.get::<_, String>(7)?).unwrap_or(TaskStatus::Queued),
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            iterm2_session_id: row.get(10)?,
        })
    })?.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(tasks)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p shepherd-core db::tests -- --nocapture`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/db/mod.rs
git commit -m "feat: add update_task_status and get_queued_tasks DB functions"
```

---

## Task 3: Create SessionMonitor — PTY Output Parser

**Files:**
- Create: `crates/shepherd-core/src/dispatch/monitor.rs`
- Create: `crates/shepherd-core/src/dispatch/mod.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Write failing tests for SessionMonitor**

Create `crates/shepherd-core/src/dispatch/mod.rs`:
```rust
pub mod monitor;
```

Create `crates/shepherd-core/src/dispatch/monitor.rs`:
```rust
//! SessionMonitor — parses PTY output to detect permission requests
//! and agent status changes based on adapter-defined patterns.

use crate::adapters::protocol::{PermissionsSection, StatusSection};
use regex::Regex;

/// What the monitor detected in the output.
#[derive(Debug, Clone, PartialEq)]
pub enum Detection {
    /// Agent is actively working.
    Working,
    /// Agent is idle / finished.
    Idle,
    /// Agent is requesting permission for an action.
    PermissionRequest { tool_name: String, tool_args: String },
    /// Agent encountered an error.
    Error(String),
    /// No pattern matched.
    None,
}

/// Monitors PTY output for a single task's agent session.
pub struct SessionMonitor {
    working_patterns: Vec<Regex>,
    idle_patterns: Vec<Regex>,
    input_patterns: Vec<Regex>,
    error_patterns: Vec<Regex>,
    approve_seq: String,
    deny_seq: String,
}

impl SessionMonitor {
    pub fn new(status: &StatusSection, permissions: &PermissionsSection) -> Self {
        let compile = |patterns: &[String]| -> Vec<Regex> {
            patterns.iter()
                .filter_map(|p| Regex::new(p).ok())
                .collect()
        };
        Self {
            working_patterns: compile(&status.working_patterns),
            idle_patterns: compile(&status.idle_patterns),
            input_patterns: compile(&status.input_patterns),
            error_patterns: compile(&status.error_patterns),
            approve_seq: permissions.approve.clone(),
            deny_seq: permissions.deny.clone(),
        }
    }

    /// Analyze a chunk of PTY output. Returns the most significant detection.
    pub fn analyze(&self, output: &str) -> Detection {
        // Check patterns in priority order: error > input > working > idle
        for re in &self.error_patterns {
            if let Some(m) = re.find(output) {
                return Detection::Error(m.as_str().to_string());
            }
        }
        for re in &self.input_patterns {
            if re.is_match(output) {
                // Extract tool info from the output if possible
                let tool_name = self.extract_tool_name(output).unwrap_or_default();
                let tool_args = self.extract_tool_args(output).unwrap_or_default();
                return Detection::PermissionRequest { tool_name, tool_args };
            }
        }
        for re in &self.working_patterns {
            if re.is_match(output) {
                return Detection::Working;
            }
        }
        for re in &self.idle_patterns {
            if re.is_match(output) {
                return Detection::Idle;
            }
        }
        Detection::None
    }

    /// Get the byte sequence to send to PTY stdin to approve.
    pub fn approve_sequence(&self) -> &str {
        &self.approve_seq
    }

    /// Get the byte sequence to send to PTY stdin to deny.
    pub fn deny_sequence(&self) -> &str {
        &self.deny_seq
    }

    fn extract_tool_name(&self, output: &str) -> Option<String> {
        // Common patterns: "Run command: ...", "Write to file: ...", "Tool: bash"
        if output.contains("bash") || output.contains("command") {
            Some("bash".to_string())
        } else if output.contains("write") || output.contains("file") {
            Some("file_write".to_string())
        } else {
            Some("unknown".to_string())
        }
    }

    fn extract_tool_args(&self, output: &str) -> Option<String> {
        // Return the raw output as args for now — adapters can refine this
        Some(output.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::protocol::{PermissionsSection, StatusSection};

    fn claude_code_status() -> StatusSection {
        StatusSection {
            working_patterns: vec![r"⠋|⠙|⠹|⠸|⠼|⠴|⠦|⠧|⠇|⠏".into()],
            idle_patterns: vec![r"\$\s*$".into()],
            input_patterns: vec![r"Allow|Do you want to".into()],
            error_patterns: vec![r"Error:|panic!|FAILED".into()],
        }
    }

    fn claude_code_permissions() -> PermissionsSection {
        PermissionsSection {
            approve: "y\n".into(),
            approve_all: "!\n".into(),
            deny: "n\n".into(),
        }
    }

    #[test]
    fn detects_working_state() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        assert_eq!(monitor.analyze("⠋ Processing files..."), Detection::Working);
    }

    #[test]
    fn detects_idle_state() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        assert_eq!(monitor.analyze("$ "), Detection::Idle);
    }

    #[test]
    fn detects_permission_request() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        match monitor.analyze("Allow bash command: cargo test?") {
            Detection::PermissionRequest { tool_name, .. } => {
                assert_eq!(tool_name, "bash");
            }
            other => panic!("Expected PermissionRequest, got {:?}", other),
        }
    }

    #[test]
    fn detects_error() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        match monitor.analyze("Error: file not found") {
            Detection::Error(msg) => assert!(msg.contains("Error")),
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn error_takes_priority_over_working() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        // Output contains both working spinner and error
        match monitor.analyze("⠋ Error: compilation failed") {
            Detection::Error(_) => {} // Error should win
            other => panic!("Expected Error priority, got {:?}", other),
        }
    }

    #[test]
    fn no_match_returns_none() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        assert_eq!(monitor.analyze("just some normal output"), Detection::None);
    }

    #[test]
    fn approve_and_deny_sequences() {
        let monitor = SessionMonitor::new(&claude_code_status(), &claude_code_permissions());
        assert_eq!(monitor.approve_sequence(), "y\n");
        assert_eq!(monitor.deny_sequence(), "n\n");
    }
}
```

- [ ] **Step 2: Add `dispatch` module to `shepherd-core/src/lib.rs`**

Add `pub mod dispatch;` to `crates/shepherd-core/src/lib.rs`.

Ensure `regex` is in `Cargo.toml` dependencies for `shepherd-core`.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p shepherd-core dispatch::monitor::tests -- --nocapture`
Expected: ALL PASS (7 tests)

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-core/src/dispatch/ crates/shepherd-core/src/lib.rs crates/shepherd-core/Cargo.toml
git commit -m "feat: add SessionMonitor for PTY output pattern detection"
```

---

## Task 4: Create TaskDispatcher — Background Dispatch Loop

**Files:**
- Modify: `crates/shepherd-core/src/dispatch/mod.rs`

- [ ] **Step 1: Write failing tests for TaskDispatcher**

Add to `crates/shepherd-core/src/dispatch/mod.rs`:

```rust
pub mod monitor;

use crate::adapters::AdapterRegistry;
use crate::coordination::LockManager;
use crate::db;
use crate::db::models::{Task, TaskStatus};
use crate::events::{ServerEvent, TaskEvent};
use crate::pty::PtyManager;
use crate::yolo::YoloEngine;
use anyhow::Result;
use monitor::{Detection, SessionMonitor};
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

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
        let adapter = self.adapters.get(&task.agent_id)
            .ok_or_else(|| anyhow::anyhow!("No adapter found for agent: {}", task.agent_id))?;

        // 2. Update status to Dispatching
        {
            let conn = self.db.lock().await;
            db::update_task_status(&conn, task.id, TaskStatus::Dispatching)?;
        }
        self.event_tx.send(ServerEvent::TaskUpdated(TaskEvent {
            id: task.id,
            title: task.title.clone(),
            agent_id: task.agent_id.clone(),
            status: "dispatching".into(),
            branch: task.branch.clone(),
            repo_path: task.repo_path.clone(),
            iterm2_session_id: task.iterm2_session_id.clone(),
        }))?;

        // 3. Build command args
        let mut args = adapter.agent.args.clone();
        // Append the task prompt as the final argument
        let prompt = if task.prompt.is_empty() {
            task.title.clone()
        } else {
            task.prompt.clone()
        };
        args.push(prompt);

        // 4. Spawn PTY session
        self.pty.spawn(
            task.id,
            &adapter.agent.command,
            &args,
            &task.repo_path,
        ).await?;

        // 5. Create SessionMonitor for this task
        let monitor = SessionMonitor::new(&adapter.status, &adapter.permissions);
        self.monitors.lock().await.insert(task.id, monitor);

        // 6. Update status to Running
        {
            let conn = self.db.lock().await;
            db::update_task_status(&conn, task.id, TaskStatus::Running)?;
        }
        self.event_tx.send(ServerEvent::TaskUpdated(TaskEvent {
            id: task.id,
            title: task.title.clone(),
            agent_id: task.agent_id.clone(),
            status: "running".into(),
            branch: task.branch.clone(),
            repo_path: task.repo_path.clone(),
            iterm2_session_id: task.iterm2_session_id.clone(),
        }))?;

        Ok(())
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
            Detection::PermissionRequest { tool_name, tool_args } => {
                // Check YOLO engine
                if self.yolo.check(tool_name, tool_args) {
                    // Auto-approve
                    let seq = monitor.approve_sequence().to_string();
                    drop(monitors);
                    self.pty.write_to_session(task_id, seq.as_bytes()).await?;
                } else {
                    // Emit permission request event for UI
                    let _ = self.event_tx.send(ServerEvent::PermissionRequested(
                        crate::events::PermissionEvent {
                            id: 0, // DB will assign real ID
                            task_id,
                            tool_name: tool_name.clone(),
                            tool_args: tool_args.clone(),
                            decision: "pending".into(),
                        },
                    ));
                    // Update task status to Input
                    let conn = self.db.lock().await;
                    let _ = db::update_task_status(&conn, task_id, TaskStatus::Input);
                }
            }
            Detection::Error(_) => {
                let conn = self.db.lock().await;
                let _ = db::update_task_status(&conn, task_id, TaskStatus::Error);
            }
            Detection::Idle => {
                // Agent finished — mark task as Done
                let conn = self.db.lock().await;
                let _ = db::update_task_status(&conn, task_id, TaskStatus::Done);
            }
            _ => {}
        }

        Ok(Some(detection))
    }

    /// Approve a pending permission — send approve sequence to PTY.
    pub async fn approve_task(&self, task_id: i64) -> Result<()> {
        let monitors = self.monitors.lock().await;
        let monitor = monitors.get(&task_id)
            .ok_or_else(|| anyhow::anyhow!("No active session for task {}", task_id))?;
        let seq = monitor.approve_sequence().to_string();
        drop(monitors);
        self.pty.write_to_session(task_id, seq.as_bytes()).await?;
        let conn = self.db.lock().await;
        db::update_task_status(&conn, task_id, TaskStatus::Running)?;
        Ok(())
    }

    /// Deny a pending permission — send deny sequence to PTY.
    pub async fn deny_task(&self, task_id: i64) -> Result<()> {
        let monitors = self.monitors.lock().await;
        let monitor = monitors.get(&task_id)
            .ok_or_else(|| anyhow::anyhow!("No active session for task {}", task_id))?;
        let seq = monitor.deny_sequence().to_string();
        drop(monitors);
        self.pty.write_to_session(task_id, seq.as_bytes()).await?;
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

    #[test]
    fn task_dispatcher_constructs() {
        // Verify the struct can be created with all dependencies
        let (tx, _rx) = broadcast::channel(16);
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        let adapters = Arc::new(AdapterRegistry::new());
        let pty = Arc::new(PtyManager::new(4, crate::pty::sandbox::SandboxProfile::disabled()));
        let yolo = Arc::new(YoloEngine::new(crate::yolo::rules::RuleSet { deny: vec![], allow: vec![] }));
        let lock_manager = Arc::new(Mutex::new(LockManager::new()));

        let _dispatcher = TaskDispatcher::new(db, adapters, pty, yolo, lock_manager, tx);
    }
}
```

- [ ] **Step 2: Check that `PtyManager` has `write_to_session`**

The existing `PtyManager` may not have a `write_to_session` method. Check `crates/shepherd-core/src/pty/mod.rs` and add it if missing:

```rust
/// Write bytes to a task's PTY stdin.
pub async fn write_to_session(&self, task_id: i64, data: &[u8]) -> Result<()> {
    let mut handles = self.handles.lock().await;
    let handle = handles.get_mut(&task_id)
        .ok_or_else(|| anyhow::anyhow!("No PTY session for task {}", task_id))?;
    handle.writer.write_all(data)
        .context("writing to PTY stdin")?;
    handle.writer.flush()
        .context("flushing PTY stdin")?;
    Ok(())
}
```

Also check that `YoloEngine` has a `check(tool_name, tool_args)` method. If not, add a simple one:

```rust
/// Check if an action should be auto-approved.
pub fn check(&self, tool_name: &str, _tool_args: &str) -> bool {
    // Check deny list first
    for deny in &self.rules.deny {
        if tool_name.contains(deny) {
            return false;
        }
    }
    // Check allow list
    for allow in &self.rules.allow {
        if tool_name.contains(allow) || allow == "*" {
            return true;
        }
    }
    false
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p shepherd-core dispatch::tests -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-core/src/dispatch/ crates/shepherd-core/src/pty/mod.rs crates/shepherd-core/src/yolo/
git commit -m "feat: add TaskDispatcher with dispatch loop and PTY output handling"
```

---

## Task 5: Create `start_server()` Reusable Function

**Files:**
- Create: `crates/shepherd-server/src/startup.rs`
- Modify: `crates/shepherd-server/src/lib.rs`
- Modify: `crates/shepherd-server/src/main.rs`

- [ ] **Step 1: Write `startup.rs` with `start_server()` and `server.json` management**

Create `crates/shepherd-server/src/startup.rs`:

```rust
//! Reusable server startup — used by both Tauri (in-process) and CLI (daemon).

use crate::state::AppState;
use crate::{build_router, ws};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shepherd_core::adapters::AdapterRegistry;
use shepherd_core::config::types::ShepherdConfig;
use shepherd_core::coordination::LockManager;
use shepherd_core::db;
use shepherd_core::dispatch::TaskDispatcher;
use shepherd_core::events::ServerEvent;
use shepherd_core::pty::sandbox::SandboxProfile;
use shepherd_core::pty::PtyManager;
use shepherd_core::yolo::rules::RuleSet;
use shepherd_core::yolo::YoloEngine;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub pid: u32,
    pub port: u16,
    pub started_at: String,
}

impl ServerInfo {
    pub fn path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".shepherd")
            .join("server.json")
    }

    pub fn write(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn read() -> Option<Self> {
        let path = Self::path();
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn remove() {
        let _ = std::fs::remove_file(Self::path());
    }
}

/// Start the Axum server and return the bound address + join handle.
/// The caller owns the Tokio runtime.
pub async fn start_server(config: ShepherdConfig) -> Result<(SocketAddr, Arc<AppState>, JoinHandle<()>)> {
    // Open DB
    let db_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".shepherd")
        .join("shepherd.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = db::open(&db_path.to_string_lossy())
        .context("opening database")?;

    // Load adapters
    let mut adapters = AdapterRegistry::new();
    let adapter_dirs = [
        dirs::home_dir().map(|h| h.join(".shepherd/adapters")),
        Some(PathBuf::from("adapters")),
    ];
    for dir in adapter_dirs.iter().flatten() {
        let _ = adapters.load_dir(dir);
    }

    // Load YOLO rules
    let rules_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".shepherd")
        .join("yolo.toml");
    let rules = if rules_path.exists() {
        let content = std::fs::read_to_string(&rules_path)?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        RuleSet::default()
    };

    // Create components
    let (event_tx, _) = broadcast::channel(1024);
    let sandbox = SandboxProfile::from_config(&config);
    let pty = PtyManager::new(config.max_agents as usize, sandbox);

    let state = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        config: config.clone(),
        adapters,
        yolo: YoloEngine::new(rules),
        pty,
        event_tx: event_tx.clone(),
        llm_provider: None,
        iterm2: None,
        cloud_client: None,
    });

    // Build router
    let router = build_router(state.clone());

    // Bind
    let addr = format!("127.0.0.1:{}", config.port);
    let listener = TcpListener::bind(&addr).await
        .with_context(|| format!("binding to {addr}"))?;
    let bound_addr = listener.local_addr()?;

    // Write server.json
    let info = ServerInfo {
        pid: std::process::id(),
        port: bound_addr.port(),
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    info.write()?;

    // Start PTY output forwarding
    let pty_rx = state.pty.subscribe_output();
    let event_tx_clone = event_tx.clone();
    tokio::spawn(async move {
        crate::forward_pty_output(pty_rx, event_tx_clone).await;
    });

    // Spawn server
    let handle = tokio::spawn(async move {
        tracing::info!("Shepherd server listening on {}", bound_addr);
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("Server error: {}", e);
        }
        ServerInfo::remove();
    });

    Ok((bound_addr, state, handle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_info_serde_roundtrip() {
        let info = ServerInfo {
            pid: 12345,
            port: 7532,
            started_at: "2026-03-19T10:00:00Z".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ServerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pid, 12345);
        assert_eq!(parsed.port, 7532);
    }
}
```

- [ ] **Step 2: Add `pub mod startup;` to `crates/shepherd-server/src/lib.rs`**

- [ ] **Step 3: Refactor `main.rs` to use `start_server()`**

Replace the manual server setup in `crates/shepherd-server/src/main.rs` with:

```rust
use anyhow::Result;
use shepherd_core::config;
use shepherd_server::startup;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = config::load_config()?;
    let (addr, _state, handle) = startup::start_server(config).await?;
    println!("Shepherd server listening on {addr}");
    handle.await?;
    Ok(())
}
```

- [ ] **Step 4: Add `dirs` and `chrono` to `shepherd-server` Cargo.toml if not present**

- [ ] **Step 5: Run `cargo build -p shepherd-server` to verify compilation**

Expected: builds without errors

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-server/
git commit -m "feat: add reusable start_server() with server.json discovery"
```

---

## Task 6: Embed Server in Tauri (Replace Child Process Spawn)

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add `shepherd-server` and `shepherd-core` dependencies to `src-tauri/Cargo.toml`**

```toml
[dependencies]
shepherd-server = { path = "../crates/shepherd-server" }
shepherd-core = { path = "../crates/shepherd-core" }
```

- [ ] **Step 2: Rewrite `src-tauri/src/lib.rs` to embed server in-process**

```rust
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
fn get_server_port(state: tauri::State<'_, ServerPort>) -> u16 {
    state.0
}

struct ServerPort(u16);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_server_port])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Load config
            let config = shepherd_core::config::load_config()
                .unwrap_or_default();
            let port = config.port;

            // Manage the port so frontend can discover it
            app.manage(ServerPort(port));

            // Start embedded server on background Tokio task
            tauri::async_runtime::spawn(async move {
                match shepherd_server::startup::start_server(config).await {
                    Ok((addr, _state, handle)) => {
                        tracing::info!("Embedded server started on {}", addr);
                        // Wait for server to finish (runs until app closes)
                        let _ = handle.await;
                    }
                    Err(e) => {
                        eprintln!("Failed to start embedded server: {}", e);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Server shuts down when Tokio runtime drops
                shepherd_server::startup::ServerInfo::remove();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running shepherd desktop");
}
```

- [ ] **Step 3: Build the Tauri app to verify compilation**

Run: `cd src-tauri && cargo build`
Expected: builds without errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/
git commit -m "feat: embed Axum server in Tauri app, replace child process spawn"
```

---

## Task 7: CLI Daemon Auto-Spawn

**Files:**
- Modify: `crates/shepherd-cli/src/main.rs`

- [ ] **Step 1: Add server discovery + daemon spawn to CLI**

Add a helper function at the top of `crates/shepherd-cli/src/main.rs`:

```rust
use std::process::Command;

/// Ensure a server is running. Returns the base URL.
async fn ensure_server(server_url: &str) -> Result<String> {
    // Try to connect
    let client = Client::new();
    match client.get(&format!("{server_url}/api/health")).send().await {
        Ok(resp) if resp.status().is_success() => return Ok(server_url.to_string()),
        _ => {}
    }

    // Check server.json for an existing daemon
    if let Some(info) = shepherd_server::startup::ServerInfo::read() {
        let url = format!("http://localhost:{}", info.port);
        match client.get(&format!("{url}/api/health")).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(url),
            _ => {
                // Stale server.json — remove it
                shepherd_server::startup::ServerInfo::remove();
            }
        }
    }

    // Spawn server daemon
    eprintln!("Starting shepherd server daemon...");
    let exe = std::env::current_exe()?;
    let server_binary = exe.parent()
        .map(|p| p.join("shepherd-server"))
        .unwrap_or_else(|| "shepherd-server".into());

    Command::new(&server_binary)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to start shepherd-server daemon. Is it installed?")?;

    // Wait for server to be ready (poll up to 5 seconds)
    for _ in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Ok(resp) = client.get(&format!("{server_url}/api/health")).send().await {
            if resp.status().is_success() {
                return Ok(server_url.to_string());
            }
        }
    }

    anyhow::bail!("Server failed to start within 5 seconds")
}
```

Modify `main()` to call `ensure_server()` before any command that needs the server:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let server = ensure_server(&cli.server).await?;
    // ... rest of command handling using `server` as base URL
}
```

- [ ] **Step 2: Add `shepherd-server` dependency to `shepherd-cli/Cargo.toml`**

```toml
[dependencies]
shepherd-server = { path = "../shepherd-server" }
```

- [ ] **Step 3: Build to verify compilation**

Run: `cargo build -p shepherd-cli`
Expected: builds without errors

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-cli/
git commit -m "feat: CLI auto-spawns server daemon if not running"
```

---

## Task 8: Wire Dispatch Loop into Server Startup

**Files:**
- Modify: `crates/shepherd-server/src/startup.rs`
- Modify: `crates/shepherd-server/src/ws.rs`

- [ ] **Step 1: Start TaskDispatcher polling loop in `start_server()`**

Add to `startup.rs` after the server spawn, before returning:

```rust
// Start TaskDispatcher polling loop
let dispatcher = Arc::new(TaskDispatcher::new(
    state.db.clone(),
    Arc::new(state.adapters.clone()), // Note: may need AdapterRegistry to be Clone
    Arc::new(state.pty.clone()),      // Note: may need PtyManager to support Arc sharing
    Arc::new(state.yolo.clone()),
    Arc::new(Mutex::new(LockManager::new())),
    event_tx.clone(),
));

let dispatcher_clone = dispatcher.clone();
tokio::spawn(async move {
    loop {
        if let Err(e) = dispatcher_clone.poll_and_dispatch().await {
            tracing::error!("Dispatch loop error: {}", e);
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
});

// Forward PTY output through dispatcher for monitoring
let dispatcher_monitor = dispatcher.clone();
let mut pty_monitor_rx = state.pty.subscribe_output();
tokio::spawn(async move {
    while let Ok(output) = pty_monitor_rx.recv().await {
        let text = String::from_utf8_lossy(&output.data);
        let _ = dispatcher_monitor.handle_pty_output(output.task_id, &text).await;
    }
});
```

- [ ] **Step 2: Wire `TaskApprove` in `ws.rs` to dispatcher**

In `crates/shepherd-server/src/ws.rs`, the `handle_client_event` function needs to route `TaskApprove` to the dispatcher. This requires passing the dispatcher to the WebSocket handler.

For now, handle approval by writing directly to PTY via state:

```rust
ClientEvent::TaskApprove { task_id } => {
    // Send approval sequence to PTY
    // The dispatcher monitor will handle status update
    if let Some(adapter) = state.adapters.get(&get_task_agent_id(state, task_id).await) {
        let seq = adapter.permissions.approve.clone();
        let _ = state.pty.write_to_session(task_id, seq.as_bytes()).await;
    }
}
```

- [ ] **Step 3: Build to verify compilation**

Run: `cargo build --workspace`
Expected: builds (may need to add `Clone` derives or `Arc` wrapping for shared components)

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-server/ crates/shepherd-core/
git commit -m "feat: wire TaskDispatcher polling loop into server startup"
```

---

## Task 9: Wire Frontend Zustand Store to Real API

**Files:**
- Create: `src/lib/tauri.ts`
- Modify: `src/store/tasks.ts`
- Modify: `src/hooks/useWebSocket.ts`

- [ ] **Step 1: Create Tauri invoke wrapper**

Create `src/lib/tauri.ts`:

```typescript
// Tauri invoke wrapper — falls back to HTTP when not in Tauri context
const isTauri = '__TAURI__' in window;

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  throw new Error(`Tauri not available for command: ${cmd}`);
}

export async function getServerPort(): Promise<number> {
  if (isTauri) {
    return invoke<number>('get_server_port');
  }
  return 9876; // fallback for dev
}
```

- [ ] **Step 2: Update `useWebSocket` hook to discover port dynamically**

In `src/hooks/useWebSocket.ts`, replace the hardcoded port with dynamic discovery:

```typescript
import { getServerPort } from '../lib/tauri';

// In the hook setup:
const port = await getServerPort();
const wsUrl = `ws://127.0.0.1:${port}/ws`;
```

- [ ] **Step 3: Update TasksSlice to use real API**

In `src/store/tasks.ts`, wire the `createTask` action to POST to the real API:

```typescript
createTask: async (task) => {
  const port = await getServerPort();
  const resp = await fetch(`http://127.0.0.1:${port}/api/tasks`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(task),
  });
  if (!resp.ok) throw new Error('Failed to create task');
  const created = await resp.json();
  set((state) => ({ tasks: [...state.tasks, created] }));
},

fetchTasks: async () => {
  const port = await getServerPort();
  const resp = await fetch(`http://127.0.0.1:${port}/api/tasks`);
  if (!resp.ok) throw new Error('Failed to fetch tasks');
  const tasks = await resp.json();
  set({ tasks });
},
```

- [ ] **Step 4: Verify with `npm run dev` that frontend builds**

Run: `cd /Users/4n6h4x0r/src/shepherd && npm run dev`
Expected: Vite dev server starts without errors

- [ ] **Step 5: Commit**

```bash
git add src/lib/tauri.ts src/store/tasks.ts src/hooks/useWebSocket.ts
git commit -m "feat: wire frontend Zustand store to real server API"
```

---

## Task 10: Wire Terminal Component to Live PTY Stream

**Files:**
- Modify: `src/features/focus/Terminal.tsx`
- Modify: `src/features/focus/PermissionPrompt.tsx`

- [ ] **Step 1: Connect Terminal to WebSocket PTY output**

In `src/features/focus/Terminal.tsx`, update the WebSocket message handler to write `terminal_output` events to the xterm instance:

```typescript
// In the useEffect that sets up WebSocket:
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.type === 'terminal_output' && msg.data.task_id === taskId) {
    terminal.write(msg.data.data);
  }
};

// Send keystrokes to PTY:
terminal.onData((data) => {
  ws.send(JSON.stringify({
    type: 'terminal_input',
    data: { task_id: taskId, data }
  }));
});
```

- [ ] **Step 2: Wire PermissionPrompt to real approve/deny**

In `src/features/focus/PermissionPrompt.tsx`, connect the approve/deny buttons to send WebSocket events:

```typescript
const handleApprove = () => {
  ws.send(JSON.stringify({
    type: 'task_approve',
    data: { task_id: taskId }
  }));
};

const handleDeny = () => {
  ws.send(JSON.stringify({
    type: 'task_cancel',
    data: { task_id: taskId }
  }));
};
```

- [ ] **Step 3: Verify Terminal component renders in dev mode**

Run: `npm run dev`
Navigate to Focus view, verify no JS errors in console.

- [ ] **Step 4: Commit**

```bash
git add src/features/focus/Terminal.tsx src/features/focus/PermissionPrompt.tsx
git commit -m "feat: wire Terminal and PermissionPrompt to live PTY stream"
```

---

## Task 11: End-to-End Integration Test

**Files:**
- No new files — manual verification

- [ ] **Step 1: Build everything**

```bash
cargo build --workspace
cd src-tauri && cargo build
```

- [ ] **Step 2: Start the Tauri app**

```bash
cargo tauri dev
```

Verify:
- App window opens showing kanban dashboard
- No "Connection refused" errors in console
- Health endpoint responds: `curl http://localhost:7532/api/health`

- [ ] **Step 3: Create a task via CLI**

```bash
cargo run -p shepherd-cli -- new "Say hello world" --agent claude-code
```

Verify:
- Task appears in kanban board UI (Queued column)
- Task transitions to Running within 2 seconds (dispatch loop picks it up)
- Terminal output streams in Focus view

- [ ] **Step 4: Test permission flow**

Create a task that triggers a permission request (e.g., file write). Verify:
- Permission prompt appears in UI
- Clicking "Approve" sends input to agent
- Task continues running

- [ ] **Step 5: Commit any fixes discovered during integration testing**

```bash
git add -A
git commit -m "fix: integration test fixes for Phase 1 core loop"
```

---

## Task 12: Run Full Test Suite and Verify Coverage

- [ ] **Step 1: Run all Rust tests**

```bash
cargo test --workspace
```

Expected: ALL PASS

- [ ] **Step 2: Run coverage**

```bash
cargo tarpaulin --config tarpaulin.toml --engine llvm
```

Expected: ≥95% (new dispatch code is testable; PTY spawn code will be excluded)

- [ ] **Step 3: Update tarpaulin.toml if needed**

Add any new files with untestable PTY/IO code to exclude-files.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "test: Phase 1 complete — full test suite passing"
git push
```
