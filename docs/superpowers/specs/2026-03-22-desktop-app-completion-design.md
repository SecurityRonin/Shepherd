# Shepherd Desktop App Completion — Design Spec

**Date:** 2026-03-22
**Goal:** Close all gaps between the existing frontend and backend to make the desktop app fully functional. Six missing API routes, a permission automation engine, Tauri desktop integration (notifications, tray, capabilities), and integration tests.

**Approach:** Feature-sliced TDD. Each feature: write Rust handler test (RED) -> implement handler (GREEN) -> verify frontend integration.

---

## 1. Missing Task Routes

### 1a. GET /api/tasks/:id

**Purpose:** Fetch a single task by ID.

**Handler:** Query SQLite `tasks` table using existing `queries::get_task(&db, id)`. Return the `Task` struct as JSON. Return 404 with `{ "error": "Task not found" }` if no row. Follow the same pattern as `approve_task` handler in `tasks.rs`.

**Route registration:** The path `/api/tasks/:id` already has a DELETE handler registered. Axum does NOT allow two separate `.route()` calls with the same path — it will panic with "overlapping route." Use method routing to combine GET and DELETE on the same path:

```rust
// Replace the existing:
//   .route("/api/tasks/:id", delete(routes::tasks::delete_task))
// With:
.route("/api/tasks/:id", get(routes::tasks::get_task).delete(routes::tasks::delete_task))
```

**Tests:**
- `get_task_returns_task` — insert a task via DB, GET it, verify fields match
- `get_task_returns_404` — GET non-existent ID, verify 404 status

### 1b. POST /api/tasks/:id/cancel

**Purpose:** Cancel a running task. Kills the PTY child process and updates DB status.

**Model change required:** The existing `TaskStatus` enum (`Queued`, `Dispatching`, `Running`, `Input`, `Review`, `Error`, `Done`) does NOT have a `Cancelled` variant. Add it:
- Add `Cancelled` variant to the enum in shepherd-core
- Update `as_str()` to return `"cancelled"`
- Update `parse_status()` / `FromStr` to parse `"cancelled"`
- Update serde tests
- Update frontend TypeScript `TaskStatus` type to include `"cancelled"`
- Add `"cancelled"` to the `STATUS_COLORS` map in `FocusView.tsx` and `SessionSidebar.tsx` (use `bg-shepherd-muted`)

**Handler:**
1. Query task by ID using `queries::get_task(&db, id)`, return 404 if missing
2. If task status is already `done`, `error`, or `cancelled`, return 400 `{ "error": "Task already finished" }`
3. Call `pty_manager.kill(task_id)` to terminate the child process. `PtyManager::kill()` already exists at `pty/mod.rs:147`. Handle the case where the process already exited gracefully (the kill may fail with "No such process" if the task completed between our status check and kill call — this TOCTOU race is expected; treat it as success)
4. Update task status to `cancelled` in DB
5. Broadcast `ServerEvent::TaskUpdated` with the new status
6. Return `{ "status": "cancelled" }`

**Tests:**
- `cancel_running_task_succeeds` — create task with "running" status, cancel it, verify status changed
- `cancel_finished_task_returns_400` — create task with "done" status, try to cancel, verify 400
- `cancel_nonexistent_task_returns_404`
- `cancel_already_exited_process_succeeds` — cancel a task whose PTY already exited, verify it still sets status to cancelled

---

## 2. Plugin Detection

### GET /api/plugins/detected

**Purpose:** Scan the host system for installed agent CLI tools and report which ones are available.

**Handler:**
1. Define a static list of known plugins with their binary names:
   ```
   claude-code -> "claude"
   aider -> "aider"
   codex -> "codex"
   goose -> "goose"
   opencode -> "opencode"
   amp -> "amp"
   cline -> "cline"
   roo -> "roo"
   ```
2. For each, check if the binary exists on PATH using `std::env::split_paths(std::env::var_os("PATH"))` combined with `std::fs::metadata` for cross-platform support (do NOT shell out to `which`)
3. Collect IDs where the binary exists
4. Return `{ "detected": ["claude-code", "aider", ...] }`

**Performance:** Run all checks concurrently with `futures::join_all`. Each check should timeout after 2 seconds.

**File:** New file `crates/shepherd-server/src/routes/plugins.rs`

**Tests:**
- `detected_plugins_returns_valid_shape` — call endpoint, verify response has `{ detected: [...] }` where detected is a string array
- `detected_plugins_with_mocked_path` — set a custom `PATH` env var pointing to a temp dir with a fake `claude` binary, verify it appears in results. This avoids environment-dependent test flakiness.

---

## 3. Replay Events

### GET /api/replay/task/:taskId

**Purpose:** Return the event history for a task, enabling the Replay Viewer to show a timeline of what happened.

**Existing infrastructure:** The `crates/shepherd-core/src/replay.rs` module already provides a complete replay system with:
- `session_events` table (columns: `id`, `task_id`, `session_id`, `event_type`, `summary`, `content`, `metadata`, `timestamp`)
- `migrate()` — creates the table
- `record_event()` — inserts events
- `get_timeline()` — queries events for a task ordered by timestamp
- `get_events_by_type()`, `search_events()`, `event_count()`, `session_duration()`

**DO NOT create a duplicate table.** The route handler should call the existing `replay::get_timeline(db, task_id)` function.

**Event recording:** Verify that the startup PTY output forwarding task (startup.rs ~line 151) calls `replay::record_event()` when broadcasting events. If not already wired, add calls for `TerminalOutput`, `TaskUpdated`, `PermissionRequested`, and `GateResult` events.

**Handler:** Call `replay::get_timeline(&db, task_id)`. Map the `SessionEvent` structs to the JSON shape expected by the frontend's `TimelineEvent` type. Return empty array (not 404) if task has no events.

**File:** New file `crates/shepherd-server/src/routes/replay.rs`

**Tests:** Since `replay.rs` already has unit tests for the query logic, the route tests only verify HTTP layer:
- `replay_returns_200_with_empty_array_for_new_task` — create task, GET replay, verify 200 + `[]`
- `replay_returns_events_in_chronological_order` — insert events via `record_event()`, GET replay, verify ordering and JSON shape

---

## 4. Permission Automation Engine

### Namespace Clarification

The existing `crates/shepherd-core/src/triggers/` module contains **UI suggestion triggers** — detectors (`NameGenDetector`, `LogoGenDetector`, `NorthStarDetector`) that scan a project and suggest tools the user might want to try. These are unrelated to permission automation.

The permission automation engine is a **separate concept**: rules that auto-approve or auto-reject agent permission requests based on patterns. To avoid namespace collision, this goes in a new module: `crates/shepherd-core/src/automation/`.

### Overview

Permission automation rules let users define patterns that automatically resolve permission requests without manual interaction. Primary use case: auto-approve safe operations (e.g., "allow all file reads in src/") so agents can work with fewer interruptions.

### Data Model

```sql
CREATE TABLE IF NOT EXISTS automation_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    rule_type TEXT NOT NULL,           -- 'auto_approve', 'auto_reject'
    pattern TEXT NOT NULL,             -- glob pattern to match against tool:path (e.g., "read_file:src/**")
    scope TEXT,                        -- optional: restrict to specific project directory (canonicalized)
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Dismissed triggers** (per-project memory of UI trigger suggestions the user chose to ignore — serves the existing trigger suggestion system):

```sql
CREATE TABLE IF NOT EXISTS trigger_dismissals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trigger_id TEXT NOT NULL,          -- matches TriggerSuggestion.id from frontend
    project_dir TEXT NOT NULL,
    dismissed_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(trigger_id, project_dir)
);
```

Note: `trigger_dismissals` serves the EXISTING trigger suggestion system (Section 4 routes), NOT the automation rules. The two systems are related but distinct:
- **Trigger suggestions** (existing `triggers/` module): "Hey, you should try the Name Generator"
- **Automation rules** (new `automation/` module): "Auto-approve file reads in src/"

### Security

**Path traversal prevention:** Before glob-matching a permission request's file path against a rule's pattern, canonicalize both paths using `std::fs::canonicalize()` or `std::path::Path::canonicalize()`. This prevents `../` traversal attacks (e.g., `src/../../etc/passwd` matching `src/**`). If canonicalization fails (file doesn't exist), reject the match.

**Directory scanning restriction:** The `POST /api/triggers/check` endpoint accepts a `project_dir` parameter. Validate that this directory:
1. Actually exists (`std::fs::metadata`)
2. Is a directory (not a file or symlink to a sensitive location)
3. Contains a `.git` directory or is listed as a known repo path in the tasks table

This prevents scanning arbitrary system directories.

### Routes

#### POST /api/triggers/check

**Request:** `{ "project_dir": "/path/to/project" }`

**Logic:**
1. Validate `project_dir` (see Security section above)
2. Delegate to the EXISTING `triggers::check_triggers()` function which runs the `TriggerDetector` implementations
3. Filter out any suggestions whose `id` appears in `trigger_dismissals` for this `project_dir`
4. Return suggestions sorted by priority

**Response:** `TriggerSuggestion[]` (matches the frontend's existing type)

#### POST /api/triggers/dismiss

**Request:** `{ "trigger_id": "...", "project_dir": "/path/to/project" }`

**Logic:** INSERT OR IGNORE into `trigger_dismissals`. Return `{ "success": true }`.

#### GET /api/automation-rules (new — not yet in frontend)

**Response:** All rules from the `automation_rules` table. Frontend can add a management UI later.

#### POST /api/automation-rules (new — not yet in frontend)

**Request:** `{ "name": "...", "rule_type": "...", "pattern": "...", "scope": "..." }`

**Logic:** Validate `rule_type` is one of `auto_approve`, `auto_reject`. If `scope` is provided, canonicalize it. INSERT into `automation_rules`. Return the created rule with its ID.

#### DELETE /api/automation-rules/:id (new — not yet in frontend)

**Logic:** DELETE from `automation_rules` WHERE `id = ?`. Return `{ "deleted": true }`.

### Integration with Dispatcher

In the TaskDispatcher's permission handling flow, add an automation evaluation step. The evaluation order matters:

1. YoloEngine evaluates first (existing behavior — handles global yolo/safe mode)
2. **NEW:** If YoloEngine defers to user, query `SELECT * FROM automation_rules WHERE enabled = 1` and check for pattern matches
3. If an `auto_approve` rule matches, resolve the permission as approved and skip user prompt
4. If an `auto_reject` rule matches, resolve as rejected
5. If no rule matches, proceed with normal user prompt flow (existing behavior)

Pattern matching: Parse the pattern as `"tool_name:path_glob"` (e.g., `"read_file:src/**"`). Match the tool name exactly, then glob-match the path using the `glob-match` crate (lightweight, no-std compatible). Canonicalize paths before matching.

**File:** New module `crates/shepherd-core/src/automation/mod.rs` with:
- `AutomationEngine` struct wrapping a DB handle
- `evaluate(tool: &str, path: &str, project_dir: &str) -> Option<AutomationDecision>` method
- `AutomationDecision` enum: `Approve`, `Reject`

**File:** New file `crates/shepherd-server/src/routes/triggers.rs` for trigger check/dismiss route handlers
**File:** New file `crates/shepherd-server/src/routes/automation.rs` for automation rule CRUD route handlers

### Tests

- `check_triggers_returns_suggestions` — create a temp dir with package.json, call check, verify suggestions returned
- `check_triggers_excludes_dismissed` — dismiss a suggestion, call check again, verify it's gone
- `check_triggers_rejects_nonexistent_dir` — pass a fake path, verify error response
- `check_triggers_rejects_non_git_dir` — pass a dir without .git, verify rejection
- `dismiss_trigger_idempotent` — dismiss same trigger twice, no error
- `create_automation_rule_and_list` — POST a rule, GET all, verify it appears
- `auto_approve_matches_pattern` — create rule `read_file:src/**`, evaluate `read_file` + `src/main.rs`, verify `Approve`
- `auto_approve_rejects_traversal` — create rule `read_file:src/**`, evaluate `read_file` + `src/../../etc/passwd`, verify no match (canonicalization blocks it)
- `auto_approve_skips_non_matching` — create rule for `read_file:src/**`, evaluate `bash` + `rm -rf /`, verify no match
- `auto_reject_matches` — create auto_reject rule, verify `Reject` decision
- `delete_automation_rule` — create and delete, verify gone from list

---

## 5. Tauri Desktop Integration

### 5a. Notification Plugin

**Cargo.toml addition:**
```toml
tauri-plugin-notification = "2"
```

**lib.rs changes:**
1. Add `.plugin(tauri_plugin_notification::init())` to the builder chain
2. Register tray commands by updating the invoke_handler:
```rust
.invoke_handler(tauri::generate_handler![
    get_server_port,
    handle_auth_callback_cmd,
    tray::set_dock_badge,
    tray::update_tray_status
])
```

### 5b. Capabilities

**Update `capabilities/default.json`:**
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

Note: `deep-link:default` may already work without explicit declaration (Tauri v2 auto-grants some plugin permissions). Verify during implementation; if deep links break without it, this addition fixes it.

### 5c. Frontend Notification Upgrade

**Update `useNotifications.ts`:** After the existing browser `Notification` call, also invoke the Tauri notification API when running in Tauri context:

```typescript
import { invoke, isTauri } from "../lib/tauri";

// In notify():
if (isTauri) {
  invoke("plugin:notification|notify", { title, body }).catch(() => {});
}
```

This provides native OS notifications (with system sounds and notification center integration) alongside the existing browser fallback. The `.catch()` silences errors when notification permission hasn't been granted yet.

### 5d. Tray Commands

The `tray.rs` module already defines `set_dock_badge` and `update_tray_status`. The only gap is registering them in the `invoke_handler` (covered in 5a above).

**Frontend wiring:** The `updateBadge()` function in `useNotifications.ts` currently only sets `document.title`. Add a Tauri invoke to set the dock badge:
```typescript
if (isTauri) {
  invoke("set_dock_badge", { count: inputCount }).catch(() => {});
}
```

### Tests

Tauri plugin integration is best tested via:
- Rust unit test: verify `set_dock_badge` and `update_tray_status` commands compile and are callable (mock the Tauri app handle)
- Frontend unit test: verify `useNotifications` calls `invoke` when `isTauri` is true (mock the tauri module)

---

## 6. Integration Tests

### Rust Integration Tests

Add `crates/shepherd-server/tests/integration.rs` with tests that:

1. **Spin up the server** using `start_server(config)` with an ephemeral port (port 0) and a temp-dir SQLite database
2. **Health check** via GET /api/health — verify 200
3. **Create a task** via POST /api/tasks — verify 201 with task ID
4. **Fetch the task** via GET /api/tasks/:id — verify fields match
5. **Cancel the task** via POST /api/tasks/:id/cancel — verify status changed
6. **Check plugins** via GET /api/plugins/detected — verify response shape
7. **Create and list automation rules** via POST/GET /api/automation-rules
8. **Check replay** via GET /api/replay/task/:taskId — verify empty array
9. **Check triggers** via POST /api/triggers/check — verify response shape
10. **Dismiss trigger** via POST /api/triggers/dismiss — verify success

### Database Migration Safety

All new tables use `CREATE TABLE IF NOT EXISTS`, which is safe for existing databases. No migration versioning system is needed for this release since all changes are additive.

---

## Implementation Order

1. **Task routes** (get_task, cancel_task + Cancelled status) — simplest, unblocks frontend task management
2. **Plugin detection** — standalone, unblocks Ecosystem view
3. **Replay events route** — builds on existing replay module, unblocks Replay view
4. **Permission automation engine** — largest piece, unblocks automation features
5. **Tauri integration** — independent of backend routes, unblocks native desktop experience
6. **Integration tests** — validates everything works end-to-end

Steps 2 and 3 are independent and can be parallelized.

## Files to Create

| File | Purpose |
|---|---|
| `crates/shepherd-server/src/routes/plugins.rs` | Plugin detection handler |
| `crates/shepherd-server/src/routes/replay.rs` | Replay events route (delegates to existing `replay.rs`) |
| `crates/shepherd-server/src/routes/triggers.rs` | Trigger check/dismiss handlers |
| `crates/shepherd-server/src/routes/automation.rs` | Automation rule CRUD handlers |
| `crates/shepherd-core/src/automation/mod.rs` | AutomationEngine, evaluate(), pattern matching |
| `crates/shepherd-server/tests/integration.rs` | Server integration tests |

## Files to Modify

| File | Changes |
|---|---|
| `crates/shepherd-server/src/lib.rs` | Register new routes; merge GET+DELETE on `/api/tasks/:id` |
| `crates/shepherd-server/src/routes/mod.rs` | Add `pub mod plugins; pub mod replay; pub mod triggers; pub mod automation;` |
| `crates/shepherd-server/src/routes/tasks.rs` | Add `get_task` and `cancel_task` handlers |
| `crates/shepherd-core/src/lib.rs` | Add `pub mod automation;` (triggers already registered) |
| `crates/shepherd-core/src/dispatch/mod.rs` | Add automation rule evaluation before permission prompt |
| `crates/shepherd-core/src/models.rs` (or equivalent) | Add `Cancelled` variant to `TaskStatus` enum |
| `crates/shepherd-server/src/startup.rs` | Verify replay event recording is wired in PTY forwarding |
| `crates/shepherd-server/src/db/` | Add `automation_rules` and `trigger_dismissals` tables |
| `src-tauri/Cargo.toml` | Add `tauri-plugin-notification` |
| `src-tauri/src/lib.rs` | Register notification plugin + tray commands |
| `src-tauri/capabilities/default.json` | Add notification + deep-link permissions |
| `src/hooks/useNotifications.ts` | Add Tauri native notification + dock badge |
| `src/types/task.ts` | Add `"cancelled"` to `TaskStatus` union type |
| `src/features/focus/FocusView.tsx` | Add `cancelled` to `STATUS_COLORS` map |
| `src/features/focus/SessionSidebar.tsx` | Add `cancelled` to `STATUS_COLORS` map |
