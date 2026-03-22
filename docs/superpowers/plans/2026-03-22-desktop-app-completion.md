# Desktop App Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close all gaps between the Shepherd frontend and backend to make the desktop app fully functional end-to-end.

**Architecture:** Feature-sliced TDD across 6 subsystems: task routes, plugin detection, replay events, permission automation, Tauri desktop integration, and integration tests. Each feature adds Rust handler tests (RED), implements the handler (GREEN), wires the route, and verifies the existing frontend connects correctly.

**Tech Stack:** Rust (Axum 0.7, rusqlite 0.31, tokio), TypeScript (React, Zustand), Tauri 2.0

**Spec:** `docs/superpowers/specs/2026-03-22-desktop-app-completion-design.md`

---

## File Structure

### Files to Create

| File | Responsibility |
|---|---|
| `crates/shepherd-server/src/routes/plugins.rs` | GET /api/plugins/detected handler |
| `crates/shepherd-server/src/routes/replay.rs` | GET /api/replay/task/:taskId handler |
| `crates/shepherd-server/src/routes/triggers.rs` | POST /api/triggers/check and /dismiss handlers |
| `crates/shepherd-server/src/routes/automation.rs` | CRUD handlers for /api/automation-rules |
| `crates/shepherd-core/src/automation/mod.rs` | AutomationEngine: rule storage, evaluate(), glob matching |
| `crates/shepherd-server/tests/integration.rs` | Full HTTP integration tests |

### Files to Modify

| File | Changes |
|---|---|
| `crates/shepherd-core/src/db/models.rs` | Add `Cancelled` variant to TaskStatus enum |
| `crates/shepherd-core/src/db/mod.rs` | Add automation_rules + trigger_dismissals table migrations |
| `crates/shepherd-core/src/lib.rs` | Add `pub mod automation;` |
| `crates/shepherd-server/src/lib.rs` | Register new routes; merge GET+DELETE on `/api/tasks/:id` |
| `crates/shepherd-server/src/routes/mod.rs` | Add `pub mod plugins; pub mod replay; pub mod triggers; pub mod automation;` |
| `crates/shepherd-server/src/routes/tasks.rs` | Add `get_task` and `cancel_task` handlers |
| `crates/shepherd-server/src/handler_tests.rs` | Add tests for new handlers |
| `src/types/task.ts` | Add `"cancelled"` to TaskStatus union |
| `src/features/focus/FocusView.tsx` | Add `cancelled` to STATUS_COLORS |
| `src/features/focus/SessionSidebar.tsx` | Add `cancelled` to STATUS_COLORS |
| `src/features/kanban/KanbanColumn.tsx` | Add `cancelled` to COLUMN_BG |
| `src/features/kanban/KanbanBoard.tsx` | Add `cancelled` to groupByStatus |
| `src/hooks/useNotifications.ts` | Add Tauri native notifications + dock badge |
| `src-tauri/Cargo.toml` | Add `tauri-plugin-notification` |
| `src-tauri/src/lib.rs` | Register notification plugin + tray commands |
| `src-tauri/capabilities/default.json` | Add notification + deep-link permissions |

---

## Task 1: Add `Cancelled` Variant to TaskStatus

**Files:**
- Modify: `crates/shepherd-core/src/db/models.rs:5-40`
- Test: `crates/shepherd-core/src/db/models.rs` (inline tests)

- [ ] **Step 1: Write the failing test for Cancelled variant**

Add to the existing `mod tests` block in `crates/shepherd-core/src/db/models.rs`:

```rust
#[test]
fn test_task_status_cancelled_variant() {
    assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
    assert_eq!(
        TaskStatus::parse_status("cancelled"),
        Some(TaskStatus::Cancelled)
    );
    let json = serde_json::to_string(&TaskStatus::Cancelled).unwrap();
    assert_eq!(json, r#""cancelled""#);
    let parsed: TaskStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, TaskStatus::Cancelled);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-core test_task_status_cancelled_variant`
Expected: FAIL — `Cancelled` is not a variant of `TaskStatus`

- [ ] **Step 3: Add Cancelled variant to TaskStatus enum**

In `crates/shepherd-core/src/db/models.rs`, add `Cancelled` after `Done` in the enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Dispatching,
    Running,
    Input,
    Review,
    Error,
    Done,
    Cancelled,
}
```

Add to `as_str()`:
```rust
Self::Cancelled => "cancelled",
```

Add to `parse_status()`:
```rust
"cancelled" => Some(Self::Cancelled),
```

Update `test_task_status_serde_roundtrip` to include `TaskStatus::Cancelled` in the `statuses` vec.

Update `test_task_status_as_str_all_variants` to add:
```rust
assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
```

- [ ] **Step 4: Run all model tests to verify they pass**

Run: `cargo test -p shepherd-core db::models::tests`
Expected: ALL PASS

- [ ] **Step 5: Update frontend TaskStatus type**

In `src/types/task.ts`, change line 1:
```typescript
export type TaskStatus = "queued" | "running" | "input" | "review" | "error" | "done" | "cancelled";
```

- [ ] **Step 6: Add `cancelled` to STATUS_COLORS in FocusView.tsx**

In `src/features/focus/FocusView.tsx`, add after the `done` entry (line 24):
```typescript
cancelled: "bg-shepherd-muted",
```

- [ ] **Step 7: Add `cancelled` to STATUS_COLORS in SessionSidebar.tsx**

In `src/features/focus/SessionSidebar.tsx`, add after the `done` entry (line 10):
```typescript
cancelled: "bg-shepherd-muted",
```

- [ ] **Step 8: Add `cancelled` to COLUMN_BG in KanbanColumn.tsx**

In `src/features/kanban/KanbanColumn.tsx`, add after the `error` entry (line 21):
```typescript
cancelled: "border-shepherd-muted/30",
```

- [ ] **Step 9: Add `cancelled` to groupByStatus in KanbanBoard.tsx**

In `src/features/kanban/KanbanBoard.tsx`, add `cancelled: []` to the `grouped` object initializer.

- [ ] **Step 10: Update frontend type test**

In `src/types/__tests__/types.test.ts`, update the TaskStatus test to include `"cancelled"`:
```typescript
const statuses: TaskStatus[] = ['queued', 'running', 'input', 'review', 'error', 'done', 'cancelled'];
```

- [ ] **Step 11: Run frontend tests**

Run: `npx vitest run src/types/__tests__/types.test.ts`
Expected: PASS

- [ ] **Step 12: Commit**

```bash
git add crates/shepherd-core/src/db/models.rs src/types/task.ts src/features/focus/FocusView.tsx src/features/focus/SessionSidebar.tsx src/features/kanban/KanbanColumn.tsx src/features/kanban/KanbanBoard.tsx src/types/__tests__/types.test.ts
git commit -m "feat: add Cancelled variant to TaskStatus enum and frontend types"
```

---

## Task 2: GET /api/tasks/:id Route

**Files:**
- Modify: `crates/shepherd-server/src/routes/tasks.rs`
- Modify: `crates/shepherd-server/src/lib.rs:19`
- Test: `crates/shepherd-server/src/handler_tests.rs`

- [ ] **Step 1: Write failing handler tests for get_task**

Add to the `mod tests` block in `crates/shepherd-server/src/handler_tests.rs`:

```rust
#[tokio::test]
async fn handler_get_task() {
    let state = test_state();
    // Create a task first
    let app = crate::build_router(state.clone());
    let resp = app
        .oneshot(json_post(
            "/api/tasks",
            serde_json::json!({
                "title": "Fetch me",
                "agent_id": "claude-code"
            }),
        ))
        .await
        .unwrap();
    let body = body_json(resp).await;
    let id = body["id"].as_i64().unwrap();

    // GET the task
    let app = crate::build_router(state);
    let resp = app
        .oneshot(get(&format!("/api/tasks/{}", id)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["title"], "Fetch me");
    assert_eq!(body["id"], id);
}

#[tokio::test]
async fn handler_get_task_not_found() {
    let state = test_state();
    let app = crate::build_router(state);
    let resp = app
        .oneshot(get("/api/tasks/99999"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn direct_get_task() {
    let state = test_state();
    // Create a task
    let input = shepherd_core::db::models::CreateTask {
        title: "Direct get".to_string(),
        agent_id: "claude-code".to_string(),
        prompt: None,
        repo_path: None,
        isolation_mode: None,
        iterm2_session_id: None,
    };
    let (_, Json(created)) =
        crate::routes::tasks::create_task(State(state.clone()), Json(input))
            .await
            .unwrap();
    let id = created["id"].as_i64().unwrap();

    // Get it
    let result = crate::routes::tasks::get_task(State(state), Path(id)).await;
    let Json(body) = result.unwrap();
    assert_eq!(body["title"], "Direct get");
}

#[tokio::test]
async fn direct_get_task_not_found() {
    let state = test_state();
    let result = crate::routes::tasks::get_task(State(state), Path(99999i64)).await;
    assert!(result.is_err());
    let (status, _) = result.unwrap_err();
    assert_eq!(status, StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p shepherd-server handler_get_task`
Expected: FAIL — `get_task` function does not exist in `routes::tasks`

- [ ] **Step 3: Implement the get_task handler**

Add to `crates/shepherd-server/src/routes/tasks.rs`, after `delete_task`:

```rust
#[tracing::instrument(skip(state))]
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let task = queries::get_task(&db, id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Task not found: {}", e) })),
        )
    })?;
    Ok(Json(serde_json::to_value(task).unwrap()))
}
```

- [ ] **Step 4: Wire the route — merge GET + DELETE on `/api/tasks/:id`**

In `crates/shepherd-server/src/lib.rs`, change line 19 from:
```rust
.route("/api/tasks/:id", delete(routes::tasks::delete_task))
```
to:
```rust
.route("/api/tasks/:id", get(routes::tasks::get_task).delete(routes::tasks::delete_task))
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p shepherd-server handler_get_task direct_get_task`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-server/src/routes/tasks.rs crates/shepherd-server/src/lib.rs crates/shepherd-server/src/handler_tests.rs
git commit -m "feat: add GET /api/tasks/:id route"
```

---

## Task 3: POST /api/tasks/:id/cancel Route

**Files:**
- Modify: `crates/shepherd-server/src/routes/tasks.rs`
- Modify: `crates/shepherd-server/src/lib.rs`
- Test: `crates/shepherd-server/src/handler_tests.rs`

- [ ] **Step 1: Write failing handler tests for cancel_task**

Add to `crates/shepherd-server/src/handler_tests.rs`:

```rust
#[tokio::test]
async fn handler_cancel_running_task() {
    let state = test_state();
    // Create a task with status "running"
    {
        let db = state.db.lock().await;
        db.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Running task', '', 'claude-code', '/tmp', 'main', 'none', 'running')",
            [],
        ).unwrap();
    }
    let app = crate::build_router(state.clone());
    let resp = app
        .oneshot(json_post("/api/tasks/1/cancel", serde_json::json!({})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "cancelled");

    // Verify DB status updated
    let db = state.db.lock().await;
    let status: String = db
        .query_row("SELECT status FROM tasks WHERE id = 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(status, "cancelled");
}

#[tokio::test]
async fn handler_cancel_finished_task_returns_400() {
    let state = test_state();
    {
        let db = state.db.lock().await;
        db.execute(
            "INSERT INTO tasks (id, title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (1, 'Done task', '', 'claude-code', '/tmp', 'main', 'none', 'done')",
            [],
        ).unwrap();
    }
    let app = crate::build_router(state);
    let resp = app
        .oneshot(json_post("/api/tasks/1/cancel", serde_json::json!({})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("already finished"));
}

#[tokio::test]
async fn handler_cancel_nonexistent_returns_404() {
    let state = test_state();
    let app = crate::build_router(state);
    let resp = app
        .oneshot(json_post("/api/tasks/99999/cancel", serde_json::json!({})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p shepherd-server handler_cancel`
Expected: FAIL — `cancel_task` function does not exist, route not registered

- [ ] **Step 3: Implement the cancel_task handler**

Add to `crates/shepherd-server/src/routes/tasks.rs`:

```rust
#[tracing::instrument(skip(state))]
pub async fn cancel_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let task = {
        let db = state.db.lock().await;
        queries::get_task(&db, id).map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("Task not found: {}", e) })),
            )
        })?
    };

    // Reject if task already finished
    match task.status {
        TaskStatus::Done | TaskStatus::Error | TaskStatus::Cancelled => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Task already finished" })),
            ));
        }
        _ => {}
    }

    // Kill the PTY process (TOCTOU: process may have already exited — treat as success)
    let _ = state.pty.kill(id).await;

    // Update status in DB
    {
        let db = state.db.lock().await;
        queries::update_task_status(&db, id, &TaskStatus::Cancelled).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;
    }

    // Broadcast status update
    let _ = state.event_tx.send(ServerEvent::TaskUpdated(TaskEvent {
        id: task.id,
        title: task.title,
        agent_id: task.agent_id,
        status: "cancelled".into(),
        branch: task.branch,
        repo_path: task.repo_path,
        iterm2_session_id: task.iterm2_session_id,
    }));

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}
```

**Note:** `PtyManager::kill(task_id: i64) -> Result<()>` exists at `crates/shepherd-core/src/pty/mod.rs:147`. The `let _ =` discards the error intentionally — the process may have already exited (TOCTOU).

- [ ] **Step 4: Register the cancel route**

Add to `crates/shepherd-server/src/lib.rs`, after the approve route (line 28):
```rust
.route("/api/tasks/:id/cancel", post(routes::tasks::cancel_task))
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p shepherd-server handler_cancel`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-server/src/routes/tasks.rs crates/shepherd-server/src/lib.rs crates/shepherd-server/src/handler_tests.rs
git commit -m "feat: add POST /api/tasks/:id/cancel route with Cancelled status"
```

---

## Task 4: GET /api/plugins/detected Route

**Files:**
- Create: `crates/shepherd-server/src/routes/plugins.rs`
- Modify: `crates/shepherd-server/src/routes/mod.rs`
- Modify: `crates/shepherd-server/src/lib.rs`
- Test: `crates/shepherd-server/src/handler_tests.rs`

- [ ] **Step 1: Write failing handler test**

Add to `crates/shepherd-server/src/handler_tests.rs`:

```rust
#[tokio::test]
async fn handler_plugins_detected_returns_valid_shape() {
    let state = test_state();
    let app = crate::build_router(state);
    let resp = app
        .oneshot(get("/api/plugins/detected"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    // Response must have "detected" array of strings
    let detected = body["detected"].as_array().unwrap();
    for item in detected {
        assert!(item.is_string());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-server handler_plugins_detected`
Expected: FAIL — 404, no such route

- [ ] **Step 3: Implement the plugins route handler**

Create `crates/shepherd-server/src/routes/plugins.rs`:

```rust
use axum::{http::StatusCode, Json};
use serde_json::Value;
use std::path::PathBuf;

/// Known agent CLI plugins and their binary names.
const KNOWN_PLUGINS: &[(&str, &str)] = &[
    ("claude-code", "claude"),
    ("aider", "aider"),
    ("codex", "codex"),
    ("goose", "goose"),
    ("opencode", "opencode"),
    ("amp", "amp"),
    ("cline", "cline"),
    ("roo", "roo"),
];

/// Check if a binary exists on PATH.
fn binary_exists_on_path(binary: &str) -> bool {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary);
            if candidate.is_file() {
                return true;
            }
            // On Windows, also check .exe
            #[cfg(windows)]
            {
                let exe = dir.join(format!("{}.exe", binary));
                if exe.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

#[tracing::instrument]
pub async fn detected_plugins() -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let detected: Vec<&str> = KNOWN_PLUGINS
        .iter()
        .filter(|(_, binary)| binary_exists_on_path(binary))
        .map(|(id, _)| *id)
        .collect();

    Ok(Json(serde_json::json!({ "detected": detected })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_exists_finds_common_tools() {
        // "ls" or "cmd" should exist on any system
        #[cfg(unix)]
        assert!(binary_exists_on_path("ls"));
        #[cfg(windows)]
        assert!(binary_exists_on_path("cmd"));
    }

    #[test]
    fn binary_exists_returns_false_for_nonexistent() {
        assert!(!binary_exists_on_path("definitely_not_a_real_binary_xyz123"));
    }

    #[test]
    fn detected_plugins_with_mocked_path() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a fake "claude" binary
        let fake_binary = tmp.path().join("claude");
        std::fs::write(&fake_binary, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake_binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Override PATH to only include our temp dir
        let old_path = std::env::var_os("PATH");
        std::env::set_var("PATH", tmp.path());

        let detected: Vec<&str> = KNOWN_PLUGINS
            .iter()
            .filter(|(_, binary)| binary_exists_on_path(binary))
            .map(|(id, _)| *id)
            .collect();

        assert!(detected.contains(&"claude-code"));
        assert!(!detected.contains(&"aider"));

        // Restore PATH
        if let Some(p) = old_path {
            std::env::set_var("PATH", p);
        }
    }
}
```

- [ ] **Step 4: Register the module and route**

Add to `crates/shepherd-server/src/routes/mod.rs`:
```rust
pub mod plugins;
```

Add to `crates/shepherd-server/src/lib.rs` (after the templates route):
```rust
.route("/api/plugins/detected", get(routes::plugins::detected_plugins))
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p shepherd-server handler_plugins_detected`
Run: `cargo test -p shepherd-server routes::plugins::tests`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-server/src/routes/plugins.rs crates/shepherd-server/src/routes/mod.rs crates/shepherd-server/src/lib.rs crates/shepherd-server/src/handler_tests.rs
git commit -m "feat: add GET /api/plugins/detected route"
```

---

## Task 5: GET /api/replay/task/:taskId Route

**Files:**
- Create: `crates/shepherd-server/src/routes/replay.rs`
- Modify: `crates/shepherd-server/src/routes/mod.rs`
- Modify: `crates/shepherd-server/src/lib.rs`
- Test: `crates/shepherd-server/src/handler_tests.rs`

- [ ] **Step 1: Write failing handler tests**

Add to `crates/shepherd-server/src/handler_tests.rs`:

```rust
#[tokio::test]
async fn handler_replay_empty_for_new_task() {
    let state = test_state();
    // Create a task
    let app = crate::build_router(state.clone());
    let resp = app
        .oneshot(json_post(
            "/api/tasks",
            serde_json::json!({ "title": "Replay test", "agent_id": "claude-code" }),
        ))
        .await
        .unwrap();
    let body = body_json(resp).await;
    let id = body["id"].as_i64().unwrap();

    // GET replay — should be empty array, not 404
    let app = crate::build_router(state);
    let resp = app
        .oneshot(get(&format!("/api/replay/task/{}", id)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn handler_replay_returns_events_in_order() {
    let state = test_state();
    // Insert events directly via replay module
    {
        let db = state.db.lock().await;
        shepherd_core::replay::record_event(
            &db, 1, 1,
            &shepherd_core::replay::EventType::SessionStart,
            "Started", "", None,
        ).unwrap();
        shepherd_core::replay::record_event(
            &db, 1, 1,
            &shepherd_core::replay::EventType::ToolCall,
            "Running cargo test", "cargo test", None,
        ).unwrap();
        shepherd_core::replay::record_event(
            &db, 1, 1,
            &shepherd_core::replay::EventType::SessionEnd,
            "Done", "", None,
        ).unwrap();
    }

    let app = crate::build_router(state);
    let resp = app
        .oneshot(get("/api/replay/task/1"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let events = body.as_array().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["event_type"], "session_start");
    assert_eq!(events[2]["event_type"], "session_end");
    // Verify JSON shape has expected fields
    assert!(events[0]["id"].is_number());
    assert!(events[0]["task_id"].is_number());
    assert!(events[0]["summary"].is_string());
    assert!(events[0]["timestamp"].is_string());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p shepherd-server handler_replay`
Expected: FAIL — 404, no such route

- [ ] **Step 3: Implement the replay route handler**

Create `crates/shepherd-server/src/routes/replay.rs`:

```rust
use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde_json::Value;
use std::sync::Arc;

use crate::state::AppState;

/// GET /api/replay/task/:taskId — returns event timeline for a task.
/// Delegates to the existing `shepherd_core::replay::get_timeline()`.
/// Returns empty array (not 404) if task has no events.
#[tracing::instrument(skip(state))]
pub async fn replay_events(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let events = shepherd_core::replay::get_timeline(&db, task_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::to_value(events).unwrap()))
}
```

- [ ] **Step 4: Register the module and route**

Add to `crates/shepherd-server/src/routes/mod.rs`:
```rust
pub mod replay;
```

Add to `crates/shepherd-server/src/lib.rs`:
```rust
.route("/api/replay/task/:taskId", get(routes::replay::replay_events))
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p shepherd-server handler_replay`
Expected: ALL PASS

- [ ] **Step 6: Verify replay event recording is wired in startup.rs**

Check `crates/shepherd-server/src/startup.rs` lines 148-156 — the PTY output forwarding task. Currently it only broadcasts `TerminalOutput` events via `pty_output_to_event()`. Verify whether `replay::record_event()` is called anywhere in the output forwarding pipeline.

**If not wired**, add replay recording to the PTY forwarding task in `startup.rs` (lines 151-156):

```rust
tokio::spawn(async move {
    while let Ok(output) = pty_rx.recv().await {
        let event = pty_output_to_event(&output);
        let _ = event_tx_clone.send(event);
        // Record terminal output to replay timeline
        let db = db_for_replay.lock().await;
        let _ = shepherd_core::replay::record_event(
            &db,
            output.task_id,
            0, // session_id — use 0 for the primary session
            &shepherd_core::replay::EventType::Output,
            "",
            &String::from_utf8_lossy(&output.data),
            None,
        );
    }
});
```

This requires passing a clone of the `db` Arc into the spawned task. Add `let db_for_replay = db.clone();` before the spawn.

**If already wired** (via `forward_pty_to_dispatcher` or the dispatcher's `handle_pty_output`), document that finding and skip this step.

- [ ] **Step 7: Commit**

```bash
git add crates/shepherd-server/src/routes/replay.rs crates/shepherd-server/src/routes/mod.rs crates/shepherd-server/src/lib.rs crates/shepherd-server/src/handler_tests.rs crates/shepherd-server/src/startup.rs
git commit -m "feat: add GET /api/replay/task/:taskId route and wire replay recording"
```

---

## Task 6: Trigger Check/Dismiss Routes

**Files:**
- Create: `crates/shepherd-server/src/routes/triggers.rs`
- Modify: `crates/shepherd-core/src/db/mod.rs` (add trigger_dismissals table)
- Modify: `crates/shepherd-server/src/routes/mod.rs`
- Modify: `crates/shepherd-server/src/lib.rs`
- Test: `crates/shepherd-server/src/handler_tests.rs`

- [ ] **Step 1: Write failing test for trigger_dismissals migration**

Add to `crates/shepherd-core/src/db/mod.rs` tests:

```rust
#[test]
fn test_trigger_dismissals_table_exists() {
    let conn = open_memory().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='trigger_dismissals'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-core test_trigger_dismissals_table_exists`
Expected: FAIL — table does not exist

- [ ] **Step 3: Add trigger_dismissals table to migration**

In `crates/shepherd-core/src/db/mod.rs`, add before `Ok(())` in the `migrate` function (after `crate::replay::migrate(conn)?;`):

```rust
// Trigger dismissals table
conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS trigger_dismissals (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        trigger_id TEXT NOT NULL,
        project_dir TEXT NOT NULL,
        dismissed_at TEXT NOT NULL DEFAULT (datetime('now')),
        UNIQUE(trigger_id, project_dir)
    );"
)?;
```

Update the `test_open_memory_creates_tables` test: increment the expected table count from 11 to 12 and add `'trigger_dismissals'` to the IN clause.

- [ ] **Step 4: Run migration tests**

Run: `cargo test -p shepherd-core test_trigger_dismissals_table_exists test_open_memory_creates_tables`
Expected: ALL PASS

- [ ] **Step 5: Write failing handler tests for triggers**

Add to `crates/shepherd-server/src/handler_tests.rs`:

```rust
#[tokio::test]
async fn handler_trigger_check_returns_suggestions() {
    let state = test_state();
    let tmp = tempfile::tempdir().unwrap();
    // Create a package.json to trigger NameGenDetector
    std::fs::write(tmp.path().join("package.json"), r#"{"name": "untitled"}"#).unwrap();
    // Create .git dir so validation passes
    std::fs::create_dir(tmp.path().join(".git")).unwrap();

    let app = crate::build_router(state);
    let resp = app
        .oneshot(json_post(
            "/api/triggers/check",
            serde_json::json!({ "project_dir": tmp.path().to_str().unwrap() }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let suggestions = body.as_array().unwrap();
    assert!(!suggestions.is_empty());
    // Verify shape
    assert!(suggestions[0]["id"].is_string());
    assert!(suggestions[0]["tool"].is_string());
}

#[tokio::test]
async fn handler_trigger_check_rejects_nonexistent_dir() {
    let state = test_state();
    let app = crate::build_router(state);
    let resp = app
        .oneshot(json_post(
            "/api/triggers/check",
            serde_json::json!({ "project_dir": "/nonexistent/path/xyz" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn handler_trigger_check_rejects_non_git_dir() {
    let state = test_state();
    let tmp = tempfile::tempdir().unwrap();
    // Directory exists but has no .git
    let app = crate::build_router(state);
    let resp = app
        .oneshot(json_post(
            "/api/triggers/check",
            serde_json::json!({ "project_dir": tmp.path().to_str().unwrap() }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("git"));
}

#[tokio::test]
async fn handler_trigger_dismiss_and_recheck() {
    let state = test_state();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("package.json"), r#"{"name": "untitled"}"#).unwrap();
    std::fs::create_dir(tmp.path().join(".git")).unwrap();
    let dir = tmp.path().to_str().unwrap();

    // Dismiss "namegen_untitled"
    let app = crate::build_router(state.clone());
    let resp = app
        .oneshot(json_post(
            "/api/triggers/dismiss",
            serde_json::json!({ "trigger_id": "namegen_untitled", "project_dir": dir }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["success"], true);

    // Check again — namegen_untitled should not appear
    let app = crate::build_router(state);
    let resp = app
        .oneshot(json_post(
            "/api/triggers/check",
            serde_json::json!({ "project_dir": dir }),
        ))
        .await
        .unwrap();
    let body = body_json(resp).await;
    let suggestions = body.as_array().unwrap();
    assert!(!suggestions.iter().any(|s| s["id"] == "namegen_untitled"));
}
```

- [ ] **Step 6: Implement the triggers route handler**

Create `crates/shepherd-server/src/routes/triggers.rs`:

```rust
use axum::{extract::State, http::StatusCode, Json};
use rusqlite::params;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct TriggerCheckRequest {
    project_dir: String,
}

#[derive(Deserialize)]
pub struct TriggerDismissRequest {
    trigger_id: String,
    project_dir: String,
}

/// POST /api/triggers/check — run detectors, filter dismissed, return suggestions.
#[tracing::instrument(skip(state))]
pub async fn check_triggers(
    State(state): State<Arc<AppState>>,
    Json(input): Json<TriggerCheckRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let project_path = Path::new(&input.project_dir);

    // Validate directory exists
    let metadata = std::fs::metadata(project_path).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Directory does not exist" })),
        )
    })?;
    if !metadata.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Path is not a directory" })),
        ));
    }

    // Validate it has .git or is a known repo
    if !project_path.join(".git").exists() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Not a git repository" })),
        ));
    }

    // Get dismissed trigger IDs for this project
    let dismissed = {
        let db = state.db.lock().await;
        let mut stmt = db
            .prepare("SELECT trigger_id FROM trigger_dismissals WHERE project_dir = ?1")
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;
        let ids: Vec<String> = stmt
            .query_map(params![input.project_dir], |row| row.get(0))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?
            .filter_map(|r| r.ok())
            .collect();
        ids
    };

    let suggestions = shepherd_core::triggers::check_triggers(project_path, &dismissed);
    Ok(Json(serde_json::to_value(suggestions).unwrap()))
}

/// POST /api/triggers/dismiss — dismiss a suggestion for a project.
#[tracing::instrument(skip(state))]
pub async fn dismiss_trigger(
    State(state): State<Arc<AppState>>,
    Json(input): Json<TriggerDismissRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    db.execute(
        "INSERT OR IGNORE INTO trigger_dismissals (trigger_id, project_dir) VALUES (?1, ?2)",
        params![input.trigger_id, input.project_dir],
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::json!({ "success": true })))
}
```

- [ ] **Step 7: Register the module and routes**

Add to `crates/shepherd-server/src/routes/mod.rs`:
```rust
pub mod triggers;
```

Add to `crates/shepherd-server/src/lib.rs`:
```rust
.route("/api/triggers/check", post(routes::triggers::check_triggers))
.route("/api/triggers/dismiss", post(routes::triggers::dismiss_trigger))
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p shepherd-server handler_trigger`
Expected: ALL PASS

- [ ] **Step 9: Commit**

```bash
git add crates/shepherd-core/src/db/mod.rs crates/shepherd-server/src/routes/triggers.rs crates/shepherd-server/src/routes/mod.rs crates/shepherd-server/src/lib.rs crates/shepherd-server/src/handler_tests.rs
git commit -m "feat: add POST /api/triggers/check and /dismiss routes"
```

---

## Task 7: Automation Rules — Core Engine

**Files:**
- Create: `crates/shepherd-core/src/automation/mod.rs`
- Modify: `crates/shepherd-core/src/lib.rs`
- Modify: `crates/shepherd-core/src/db/mod.rs`
- Modify: `crates/shepherd-core/Cargo.toml` (glob-match crate)

- [ ] **Step 1: Add glob-match dependency**

In `crates/shepherd-core/Cargo.toml`, add to `[dependencies]`:
```toml
glob-match = "0.2"
```

Note: The crate already has `glob = "0.3"`, but `glob-match` is a lightweight alternative for pattern matching strings (not filesystem globbing). If you prefer to use the existing `glob` crate's `Pattern::matches()` instead, that works too — in that case skip this step and use `glob::Pattern` in the implementation.

- [ ] **Step 2: Write failing tests for AutomationEngine**

Create `crates/shepherd-core/src/automation/mod.rs`:

```rust
use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: i64,
    pub name: String,
    pub rule_type: String,
    pub pattern: String,
    pub scope: Option<String>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AutomationDecision {
    Approve,
    Reject,
}

pub struct AutomationEngine;

impl AutomationEngine {
    /// Evaluate a permission request against all enabled rules.
    /// Canonicalizes path to prevent traversal attacks (e.g., `src/../../etc/passwd`).
    /// Returns the first matching decision, or None if no rule matches.
    pub fn evaluate(
        conn: &Connection,
        tool: &str,
        path: &str,
        project_dir: &str,
    ) -> Result<Option<AutomationDecision>> {
        todo!()
    }

    /// List all automation rules.
    pub fn list_rules(conn: &Connection) -> Result<Vec<AutomationRule>> {
        todo!()
    }

    /// Create a new automation rule. Returns the created rule.
    pub fn create_rule(
        conn: &Connection,
        name: &str,
        rule_type: &str,
        pattern: &str,
        scope: Option<&str>,
    ) -> Result<AutomationRule> {
        todo!()
    }

    /// Delete a rule by ID. Returns true if a row was deleted.
    pub fn delete_rule(conn: &Connection, id: i64) -> Result<bool> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    #[test]
    fn create_rule_and_list() {
        let conn = open_memory().unwrap();
        let rule = AutomationEngine::create_rule(
            &conn,
            "Allow src reads",
            "auto_approve",
            "read_file:src/**",
            None,
        )
        .unwrap();
        assert_eq!(rule.name, "Allow src reads");
        assert_eq!(rule.rule_type, "auto_approve");
        assert!(rule.enabled);

        let rules = AutomationEngine::list_rules(&conn).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, rule.id);
    }

    #[test]
    fn auto_approve_matches_pattern() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow src reads",
            "auto_approve",
            "read_file:src/**",
            None,
        )
        .unwrap();

        let decision = AutomationEngine::evaluate(&conn, "read_file", "src/main.rs", "/project")
            .unwrap();
        assert_eq!(decision, Some(AutomationDecision::Approve));
    }

    #[test]
    fn auto_reject_matches() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Block bash",
            "auto_reject",
            "bash:**",
            None,
        )
        .unwrap();

        let decision = AutomationEngine::evaluate(&conn, "bash", "rm -rf /", "/project")
            .unwrap();
        assert_eq!(decision, Some(AutomationDecision::Reject));
    }

    #[test]
    fn no_rule_matches_returns_none() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow src reads",
            "auto_approve",
            "read_file:src/**",
            None,
        )
        .unwrap();

        let decision = AutomationEngine::evaluate(&conn, "bash", "ls", "/project")
            .unwrap();
        assert_eq!(decision, None);
    }

    #[test]
    fn delete_rule_removes_it() {
        let conn = open_memory().unwrap();
        let rule = AutomationEngine::create_rule(
            &conn,
            "Temp rule",
            "auto_approve",
            "read_file:**",
            None,
        )
        .unwrap();

        assert!(AutomationEngine::delete_rule(&conn, rule.id).unwrap());
        assert!(AutomationEngine::list_rules(&conn).unwrap().is_empty());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let conn = open_memory().unwrap();
        assert!(!AutomationEngine::delete_rule(&conn, 99999).unwrap());
    }

    #[test]
    fn auto_approve_rejects_traversal() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow src reads",
            "auto_approve",
            "read_file:src/**",
            None,
        )
        .unwrap();

        // Path traversal: src/../../etc/passwd should NOT match src/**
        let decision = AutomationEngine::evaluate(
            &conn, "read_file", "src/../../etc/passwd", "/project",
        )
        .unwrap();
        assert_eq!(decision, None);
    }

    #[test]
    fn scope_restricts_rule_to_project() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow src reads for project-a",
            "auto_approve",
            "read_file:src/**",
            Some("/projects/a"),
        )
        .unwrap();

        // Should match when project_dir matches scope
        let decision = AutomationEngine::evaluate(
            &conn, "read_file", "src/main.rs", "/projects/a",
        )
        .unwrap();
        assert_eq!(decision, Some(AutomationDecision::Approve));

        // Should NOT match when project_dir differs from scope
        let decision = AutomationEngine::evaluate(
            &conn, "read_file", "src/main.rs", "/projects/b",
        )
        .unwrap();
        assert_eq!(decision, None);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p shepherd-core automation::tests`
Expected: FAIL — `todo!()` panics

- [ ] **Step 4: Add automation_rules table to migration**

In `crates/shepherd-core/src/db/mod.rs`, add after the `trigger_dismissals` table creation:

```rust
// Automation rules table
conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS automation_rules (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        rule_type TEXT NOT NULL,
        pattern TEXT NOT NULL,
        scope TEXT,
        enabled INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );"
)?;
```

Update `test_open_memory_creates_tables`: increment expected count from 12 to 13 and add `'automation_rules'` to the IN clause.

- [ ] **Step 5: Register the automation module**

Add to `crates/shepherd-core/src/lib.rs`:
```rust
pub mod automation;
```

- [ ] **Step 6: Implement AutomationEngine methods**

Replace the `todo!()`s in `crates/shepherd-core/src/automation/mod.rs`:

```rust
impl AutomationEngine {
    pub fn evaluate(
        conn: &Connection,
        tool: &str,
        path: &str,
        project_dir: &str,
    ) -> Result<Option<AutomationDecision>> {
        // Canonicalize path to prevent traversal attacks (e.g., src/../../etc/passwd)
        use std::path::Path;
        let clean_path = Path::new(path);
        let mut components = Vec::new();
        for component in clean_path.components() {
            match component {
                std::path::Component::ParentDir => {
                    // Path traversal detected — strip the `..` and the preceding component
                    components.pop();
                }
                std::path::Component::Normal(c) => {
                    components.push(c.to_string_lossy().to_string());
                }
                _ => {}
            }
        }
        let canonical_path = components.join("/");

        // Filter by scope: rules with NULL scope match all projects,
        // rules with a scope only match that specific project_dir
        let mut stmt = conn.prepare(
            "SELECT rule_type, pattern FROM automation_rules WHERE enabled = 1 AND (scope IS NULL OR scope = ?1)"
        )?;

        let rules: Vec<(String, String)> = stmt
            .query_map(params![project_dir], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let request_str = format!("{}:{}", tool, canonical_path);

        for (rule_type, pattern) in &rules {
            if glob_match::glob_match(pattern, &request_str) {
                return Ok(Some(match rule_type.as_str() {
                    "auto_approve" => AutomationDecision::Approve,
                    "auto_reject" => AutomationDecision::Reject,
                    _ => continue,
                }));
            }
        }

        Ok(None)
    }

    pub fn list_rules(conn: &Connection) -> Result<Vec<AutomationRule>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, rule_type, pattern, scope, enabled, created_at FROM automation_rules ORDER BY id"
        )?;
        let rules = stmt
            .query_map([], |row| {
                Ok(AutomationRule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    rule_type: row.get(2)?,
                    pattern: row.get(3)?,
                    scope: row.get(4)?,
                    enabled: row.get::<_, i64>(5)? == 1,
                    created_at: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rules)
    }

    pub fn create_rule(
        conn: &Connection,
        name: &str,
        rule_type: &str,
        pattern: &str,
        scope: Option<&str>,
    ) -> Result<AutomationRule> {
        conn.execute(
            "INSERT INTO automation_rules (name, rule_type, pattern, scope) VALUES (?1, ?2, ?3, ?4)",
            params![name, rule_type, pattern, scope],
        )?;
        let id = conn.last_insert_rowid();
        let rule = conn.query_row(
            "SELECT id, name, rule_type, pattern, scope, enabled, created_at FROM automation_rules WHERE id = ?1",
            params![id],
            |row| {
                Ok(AutomationRule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    rule_type: row.get(2)?,
                    pattern: row.get(3)?,
                    scope: row.get(4)?,
                    enabled: row.get::<_, i64>(5)? == 1,
                    created_at: row.get(6)?,
                })
            },
        )?;
        Ok(rule)
    }

    pub fn delete_rule(conn: &Connection, id: i64) -> Result<bool> {
        let affected = conn.execute(
            "DELETE FROM automation_rules WHERE id = ?1",
            params![id],
        )?;
        Ok(affected > 0)
    }
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p shepherd-core automation::tests`
Expected: ALL PASS

- [ ] **Step 8: Commit**

```bash
git add crates/shepherd-core/Cargo.toml crates/shepherd-core/src/automation/mod.rs crates/shepherd-core/src/lib.rs crates/shepherd-core/src/db/mod.rs
git commit -m "feat: add AutomationEngine with rule CRUD and glob pattern matching"
```

---

## Task 8: Automation Rules — HTTP Routes

**Files:**
- Create: `crates/shepherd-server/src/routes/automation.rs`
- Modify: `crates/shepherd-server/src/routes/mod.rs`
- Modify: `crates/shepherd-server/src/lib.rs`
- Test: `crates/shepherd-server/src/handler_tests.rs`

- [ ] **Step 1: Write failing handler tests**

Add to `crates/shepherd-server/src/handler_tests.rs`:

```rust
#[tokio::test]
async fn handler_automation_rules_crud() {
    let state = test_state();

    // List rules — should be empty
    let app = crate::build_router(state.clone());
    let resp = app.oneshot(get("/api/automation-rules")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);

    // Create a rule
    let app = crate::build_router(state.clone());
    let resp = app
        .oneshot(json_post(
            "/api/automation-rules",
            serde_json::json!({
                "name": "Allow src reads",
                "rule_type": "auto_approve",
                "pattern": "read_file:src/**"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let rule_id = body["id"].as_i64().unwrap();
    assert_eq!(body["name"], "Allow src reads");

    // List again — should have 1
    let app = crate::build_router(state.clone());
    let resp = app.oneshot(get("/api/automation-rules")).await.unwrap();
    let body = body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Delete the rule
    let app = crate::build_router(state.clone());
    let resp = app
        .oneshot(delete(&format!("/api/automation-rules/{}", rule_id)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["deleted"], true);

    // List again — should be empty
    let app = crate::build_router(state);
    let resp = app.oneshot(get("/api/automation-rules")).await.unwrap();
    let body = body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p shepherd-server handler_automation_rules`
Expected: FAIL — no such routes

- [ ] **Step 3: Implement the automation route handlers**

Create `crates/shepherd-server/src/routes/automation.rs`:

```rust
use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::state::AppState;
use shepherd_core::automation::AutomationEngine;

#[derive(Deserialize)]
pub struct CreateRuleRequest {
    name: String,
    rule_type: String,
    pattern: String,
    scope: Option<String>,
}

/// GET /api/automation-rules — list all rules.
#[tracing::instrument(skip(state))]
pub async fn list_rules(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let rules = AutomationEngine::list_rules(&db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::to_value(rules).unwrap()))
}

/// POST /api/automation-rules — create a new rule.
#[tracing::instrument(skip(state, input))]
pub async fn create_rule(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateRuleRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Validate rule_type
    if input.rule_type != "auto_approve" && input.rule_type != "auto_reject" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "rule_type must be 'auto_approve' or 'auto_reject'" })),
        ));
    }

    let db = state.db.lock().await;
    let rule = AutomationEngine::create_rule(
        &db,
        &input.name,
        &input.rule_type,
        &input.pattern,
        input.scope.as_deref(),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok((StatusCode::CREATED, Json(serde_json::to_value(rule).unwrap())))
}

/// DELETE /api/automation-rules/:id — delete a rule.
#[tracing::instrument(skip(state))]
pub async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let deleted = AutomationEngine::delete_rule(&db, id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
```

- [ ] **Step 4: Register the module and routes**

Add to `crates/shepherd-server/src/routes/mod.rs`:
```rust
pub mod automation;
```

Add to `crates/shepherd-server/src/lib.rs`:
```rust
.route("/api/automation-rules", get(routes::automation::list_rules).post(routes::automation::create_rule))
.route("/api/automation-rules/:id", delete(routes::automation::delete_rule))
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p shepherd-server handler_automation_rules`
Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-server/src/routes/automation.rs crates/shepherd-server/src/routes/mod.rs crates/shepherd-server/src/lib.rs crates/shepherd-server/src/handler_tests.rs
git commit -m "feat: add CRUD routes for /api/automation-rules"
```

---

## Task 9: Wire AutomationEngine into Dispatcher Permission Flow

**Files:**
- Modify: `crates/shepherd-core/src/dispatch/mod.rs:336-363`
- Test: `crates/shepherd-core/src/dispatch/mod.rs` (existing test infrastructure)

The dispatcher currently handles permission requests in `handle_pty_output()` (lines 336-363):
1. YoloEngine evaluates → `Allow` → auto-approve via PTY
2. Otherwise → emit `PermissionRequested` event, set status to `Input`

We need to insert AutomationEngine evaluation between steps 1 and 2:
1. YoloEngine evaluates → `Allow` → auto-approve
2. **AutomationEngine evaluates → `Approve` → auto-approve; `Reject` → auto-reject**
3. Otherwise → prompt user

- [ ] **Step 1: Write failing test for automation-based auto-approve**

Add to dispatch tests (in `crates/shepherd-core/src/dispatch/mod.rs` or a separate test file in the same module):

```rust
#[tokio::test]
async fn handle_pty_output_auto_approves_via_automation_rule() {
    // Setup: create dispatcher with an automation rule that auto-approves read_file:src/**
    // Insert a task into DB, create a SessionMonitor that detects permission requests
    // Feed PTY output that triggers a permission request for read_file:src/main.rs
    // Assert: PTY receives approve sequence (not PermissionRequested event)
    // This test validates the integration point between dispatcher and AutomationEngine
}
```

**Note:** The exact test setup depends on the existing dispatch test infrastructure. The key assertion is: when an automation rule matches, the dispatcher auto-approves without emitting `PermissionRequested` and without setting status to `Input`. If the dispatch module doesn't have an existing test that creates a `SessionMonitor` for permission detection, add a simpler unit test for the evaluation call:

```rust
#[tokio::test]
async fn dispatcher_calls_automation_evaluate() {
    // Create in-memory DB with automation rule
    let conn = crate::db::open_memory().unwrap();
    crate::automation::AutomationEngine::create_rule(
        &conn, "Allow src reads", "auto_approve", "read_file:src/**", None,
    ).unwrap();

    // Verify AutomationEngine returns Approve for this tool+path
    let decision = crate::automation::AutomationEngine::evaluate(
        &conn, "read_file", "src/main.rs", "/project",
    ).unwrap();
    assert_eq!(decision, Some(crate::automation::AutomationDecision::Approve));
}
```

- [ ] **Step 2: Modify the permission handling in handle_pty_output()**

In `crates/shepherd-core/src/dispatch/mod.rs`, change the permission request handling block (lines 336-363). Replace:

```rust
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
            ...
        }
    }
}
```

With:

```rust
Detection::PermissionRequest {
    tool_name,
    tool_args,
} => {
    match self.yolo.evaluate(tool_name, tool_args) {
        Decision::Allow(_) => {
            // Auto-approve via YOLO rules
            let seq = monitor.approve_sequence().to_string();
            drop(monitors);
            self.pty.write_to(task_id, &seq).await?;
        }
        _ => {
            // Check automation rules before prompting user
            let auto_decision = {
                let conn = self.db.lock().await;
                // Get repo_path for scope filtering
                let repo_path: String = conn
                    .query_row(
                        "SELECT repo_path FROM tasks WHERE id = ?1",
                        rusqlite::params![task_id],
                        |row| row.get(0),
                    )
                    .unwrap_or_default();
                crate::automation::AutomationEngine::evaluate(
                    &conn, tool_name, tool_args, &repo_path,
                )
                .unwrap_or(None)
            };

            match auto_decision {
                Some(crate::automation::AutomationDecision::Approve) => {
                    let seq = monitor.approve_sequence().to_string();
                    drop(monitors);
                    self.pty.write_to(task_id, &seq).await?;
                }
                Some(crate::automation::AutomationDecision::Reject) => {
                    // Auto-reject: record as denied, emit event
                    let _ = self.event_tx.send(ServerEvent::PermissionResolved(
                        PermissionEvent {
                            id: 0,
                            task_id,
                            tool_name: tool_name.clone(),
                            tool_args: tool_args.clone(),
                            decision: "denied".into(),
                        },
                    ));
                    drop(monitors);
                    // Don't write to PTY — agent will see permission denied
                }
                None => {
                    // No automation rule matched — prompt user
                    let _ = self.event_tx.send(ServerEvent::PermissionRequested(
                        PermissionEvent {
                            id: 0,
                            task_id,
                            tool_name: tool_name.clone(),
                            tool_args: tool_args.clone(),
                            decision: "pending".into(),
                        },
                    ));
                    drop(monitors);
                    // Update task status to Input
                    let conn = self.db.lock().await;
                    let _ = db::update_task_status(&conn, task_id, TaskStatus::Input);
                }
            }
        }
    }
}
```

- [ ] **Step 3: Run dispatch tests**

Run: `cargo test -p shepherd-core dispatch`
Expected: ALL PASS

- [ ] **Step 4: Run full workspace tests**

Run: `cargo test --workspace`
Expected: ALL PASS — no regressions

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/dispatch/mod.rs
git commit -m "feat: wire AutomationEngine into dispatcher permission flow"
```

---

## Task 10: Tauri Desktop Integration

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src/hooks/useNotifications.ts`
- Test: `src/hooks/__tests__/useNotifications.test.ts` (if exists, add tests)

- [ ] **Step 1: Add tauri-plugin-notification dependency**

In `src-tauri/Cargo.toml`, add to `[dependencies]`:
```toml
tauri-plugin-notification = "2"
```

- [ ] **Step 2: Register notification plugin and tray commands in lib.rs**

In `src-tauri/src/lib.rs`:

1. Add `.plugin(tauri_plugin_notification::init())` after `tauri_plugin_deep_link::init()`:
```rust
.plugin(tauri_plugin_notification::init())
```

2. Update `.invoke_handler` to include tray commands:
```rust
.invoke_handler(tauri::generate_handler![
    get_server_port,
    handle_auth_callback_cmd,
    tray::set_dock_badge,
    tray::update_tray_status
])
```

3. Remove the `#[allow(unused)]` attribute on `mod tray;` and the "Plan 3" comment.

- [ ] **Step 3: Update capabilities**

Replace `src-tauri/capabilities/default.json`:
```json
{
  "$schema": "https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-utils/schema.json",
  "identifier": "default",
  "description": "Default capabilities for Shepherd",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open",
    "deep-link:default",
    "notification:default",
    "notification:allow-notify",
    "notification:allow-request-permission"
  ]
}
```

- [ ] **Step 4: Add Tauri native notifications to useNotifications.ts**

In `src/hooks/useNotifications.ts`:

Add import at top:
```typescript
import { invoke } from "../lib/tauri";
```

Update the `notify` function to also invoke Tauri notification when available:
```typescript
const isTauri = typeof window !== "undefined" && "__TAURI__" in window;

function notify(title: string, body: string): void {
  if (typeof window !== "undefined" && "Notification" in window) {
    if (Notification.permission === "granted") {
      new Notification(title, { body });
    }
  }
  // Tauri native notification (system notification center, sounds)
  if (isTauri) {
    invoke("plugin:notification|notify", { title, body }).catch(() => {});
  }
}
```

Update the `updateBadge` function to also set the dock badge:
```typescript
function updateBadge(inputCount: number): void {
  if (typeof document !== "undefined") {
    document.title = inputCount > 0 ? `(${inputCount}) Shepherd` : "Shepherd";
  }
  if (isTauri) {
    invoke("set_dock_badge", { text: inputCount > 0 ? String(inputCount) : "" }).catch(() => {});
  }
}
```

- [ ] **Step 5: Run frontend tests**

Run: `npx vitest run src/hooks/`
Expected: PASS (existing tests should still pass; `invoke` is mocked)

- [ ] **Step 6: Verify Tauri build compiles**

Run: `cd src-tauri && cargo check`
Expected: Compilation succeeds (no need to build full binary)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/capabilities/default.json src/hooks/useNotifications.ts
git commit -m "feat: wire Tauri notification plugin, tray commands, and native notifications"
```

---

## Task 11: Integration Tests

**Files:**
- Create: `crates/shepherd-server/tests/integration.rs`

- [ ] **Step 1: Write the integration test file**

Create `crates/shepherd-server/tests/integration.rs`:

```rust
//! Full HTTP integration tests — spin up a real server on an ephemeral port
//! and exercise the entire request→DB→response pipeline.

use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

/// Start the server and return (base_url, the server JoinHandle).
async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let mut config = shepherd_core::config::types::ShepherdConfig::default();
    config.port = 0; // ephemeral port

    let (addr, _state, handle) = shepherd_server::startup::start_server(config)
        .await
        .expect("Failed to start test server");

    let base = format!("http://{}", addr);
    let join = tokio::spawn(async move {
        let _ = handle.await;
    });

    // Wait for server to be ready
    let client = Client::new();
    for _ in 0..20 {
        if client.get(&format!("{}/api/health", base)).send().await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    (base, join)
}

#[tokio::test]
async fn integration_health_check() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    let resp = client.get(&format!("{}/api/health", base)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn integration_task_lifecycle() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    // Create task
    let resp = client
        .post(&format!("{}/api/tasks", base))
        .json(&json!({ "title": "Integration test", "agent_id": "claude-code" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let task: Value = resp.json().await.unwrap();
    let id = task["id"].as_i64().unwrap();
    assert_eq!(task["status"], "queued");

    // Fetch task
    let resp = client
        .get(&format!("{}/api/tasks/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let fetched: Value = resp.json().await.unwrap();
    assert_eq!(fetched["title"], "Integration test");

    // Cancel task
    let resp = client
        .post(&format!("{}/api/tasks/{}/cancel", base, id))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "cancelled");

    // Verify cancelled — can't cancel again
    let resp = client
        .post(&format!("{}/api/tasks/{}/cancel", base, id))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Delete task
    let resp = client
        .delete(&format!("{}/api/tasks/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn integration_plugins_detected() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    let resp = client
        .get(&format!("{}/api/plugins/detected", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["detected"].is_array());
}

#[tokio::test]
async fn integration_replay_empty() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    // Create a task first
    let resp = client
        .post(&format!("{}/api/tasks", base))
        .json(&json!({ "title": "Replay test", "agent_id": "claude-code" }))
        .send()
        .await
        .unwrap();
    let task: Value = resp.json().await.unwrap();
    let id = task["id"].as_i64().unwrap();

    let resp = client
        .get(&format!("{}/api/replay/task/{}", base, id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn integration_automation_rules_lifecycle() {
    let (base, _handle) = start_test_server().await;
    let client = Client::new();

    // List — empty
    let resp = client
        .get(&format!("{}/api/automation-rules", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);

    // Create
    let resp = client
        .post(&format!("{}/api/automation-rules", base))
        .json(&json!({
            "name": "Test rule",
            "rule_type": "auto_approve",
            "pattern": "read_file:src/**"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let rule: Value = resp.json().await.unwrap();
    let rule_id = rule["id"].as_i64().unwrap();

    // List — should have 1
    let resp = client
        .get(&format!("{}/api/automation-rules", base))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Delete
    let resp = client
        .delete(&format!("{}/api/automation-rules/{}", base, rule_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p shepherd-server --test integration`
Expected: ALL PASS

- [ ] **Step 3: Commit**

```bash
git add crates/shepherd-server/tests/integration.rs
git commit -m "test: add full HTTP integration tests for all new routes"
```

---

## Task 12: Final Verification

- [ ] **Step 1: Run all Rust tests**

Run: `cargo test --workspace`
Expected: ALL PASS

- [ ] **Step 2: Run all frontend unit tests**

Run: `npx vitest run`
Expected: ALL PASS (436+ tests)

- [ ] **Step 3: Verify TypeScript build**

Run: `npm run build`
Expected: Build succeeds with no errors

- [ ] **Step 4: Run E2E tests**

Run: `npx playwright test`
Expected: ALL PASS (91+ tests)

- [ ] **Step 5: Final commit if any fixups needed**

```bash
git add -A && git commit -m "fix: address any test/build issues from desktop completion"
```

---

## Dependency Graph

```
Task 1 (Cancelled variant) ──► Task 3 (cancel route depends on Cancelled)
                              ▲
Task 2 (get_task route) ──────┘ (same file, merge GET+DELETE)

Task 4 (plugins) ────────────► (independent)
Task 5 (replay) ─────────────► (independent)
Task 6 (triggers) ───────────► Task 7 (automation core, shares migration)
Task 7 (automation core) ────► Task 8 (automation routes) ──► Task 9 (dispatcher wiring)
Task 10 (Tauri) ─────────────► (independent, can run in parallel with 4-9)

Tasks 1-10 all ──────────────► Task 11 (integration tests)
Task 11 ─────────────────────► Task 12 (final verification)
```

**Parallelizable groups:**
- Tasks 4, 5, 10 can run in parallel (independent)
- Tasks 6 → 7 → 8 → 9 must be sequential (shared module)
- Tasks 2, 3 can be done in either order but both depend on Task 1
