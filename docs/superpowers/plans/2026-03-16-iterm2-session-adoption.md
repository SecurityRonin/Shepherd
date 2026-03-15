# iTerm2 Session Adoption Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adopt existing iTerm2 tabs running `claude` as first-class Shepherd tasks with bidirectional I/O, gate enforcement, and session management.

**Architecture:** A new `iterm2` module in `shepherd-core` connects to iTerm2's WebSocket API over a Unix domain socket, polls for `claude` sessions every 5 s, adopts them as tasks in the DB, and streams terminal output as `ServerEvent::TerminalOutput`. Three new Axum endpoints (`/resume`, `/fresh`, `/claude-sessions`) handle session lifecycle. The frontend adds a source badge and session picker to adopted task cards.

**Tech Stack:** `prost 0.13` + `prost-build 0.13` (protobuf codegen via `build.rs`), `tokio-tungstenite 0.24` (WebSocket over `tokio::net::UnixStream`), `http 1.0` (WebSocket handshake builder), `notify 6` (credential file watching), Axum 0.7 (new endpoints), React/Zustand (badge + session picker)

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/shepherd-core/build.rs` | Create | Compile vendored iTerm2 proto with prost_build |
| `crates/shepherd-core/proto/iterm2-api.proto` | Create | Vendored iTerm2 protobuf schema (pinned) |
| `crates/shepherd-core/Cargo.toml` | Modify | Add prost, tokio-tungstenite, http, notify, futures-util deps |
| `crates/shepherd-core/src/iterm2/mod.rs` | Create | `Iterm2Manager`: public API, background scan loop |
| `crates/shepherd-core/src/iterm2/auth.rs` | Create | Load + watch `~/.shepherd/iterm2-auth.json` |
| `crates/shepherd-core/src/iterm2/client.rs` | Create | `Iterm2Transport` trait + `WsClient` impl; binary frame framing |
| `crates/shepherd-core/src/iterm2/scanner.rs` | Create | Session discovery: `ListSessions` → `VariableRequest` → `AdoptionCandidate` |
| `crates/shepherd-core/src/iterm2/session.rs` | Create | `AdoptedSession`: 50 ms debounce, cell flatten, gate detect, send text |
| `crates/shepherd-core/src/lib.rs` | Modify | Add `pub mod iterm2` |
| `crates/shepherd-core/src/db/mod.rs` | Modify | Migration: `iterm2_session_id TEXT` column on tasks table |
| `crates/shepherd-core/src/db/models.rs` | Modify | `iterm2_session_id: Option<String>` on `Task` + `CreateTask` |
| `crates/shepherd-core/src/db/queries.rs` | Modify | Add `find_task_by_iterm2_id`, `update_task_status` |
| `crates/shepherd-server/src/state.rs` | Modify | Add `iterm2: Option<Arc<Iterm2Manager>>` |
| `crates/shepherd-server/src/lib.rs` | Modify | Register 3 new `/api/sessions/:id/*` routes |
| `crates/shepherd-server/src/main.rs` | Modify | Init `Iterm2Manager`, spawn adoption background task |
| `crates/shepherd-server/src/routes/iterm2.rs` | Create | Handlers: `resume_session`, `fresh_session`, `list_claude_sessions` |
| `crates/shepherd-server/src/routes/mod.rs` | Modify | `pub mod iterm2` |
| `src/store/tasks.ts` | Modify | Add `iterm2_session_id?: string` to frontend `Task` type |
| `src/features/iterm2/Iterm2Badge.tsx` | Create | Small pill badge rendered on task cards |
| `src/features/iterm2/SessionPicker.tsx` | Create | Dropdown + Resume/Start Fresh buttons |
| `src/features/iterm2/SetupPrompt.tsx` | Create | One-time callout when auth file is absent |
| `src/features/iterm2/__tests__/iterm2.test.tsx` | Create | Component tests (Vitest + Testing Library) |

---

## Chunk 1: Foundation

### Task 1: Vendor iTerm2 proto and create build.rs

**Files:**
- Create: `crates/shepherd-core/proto/iterm2-api.proto`
- Create: `crates/shepherd-core/build.rs`

- [ ] **Step 1: Create proto directory and vendor the iTerm2 API proto**

```bash
mkdir -p crates/shepherd-core/proto
curl -fsSL \
  "https://raw.githubusercontent.com/gnachman/iTerm2/master/api/library/python/iterm2/iterm2/proto/api.proto" \
  -o crates/shepherd-core/proto/iterm2-api.proto
```

Verify the file starts with `syntax = "proto2";` and contains `package iterm2;`. The file is ~1500 lines and defines `ClientOriginatedMessage`, `ServerOriginatedMessage`, `ListSessionsRequest`, `GetBufferRequest`, etc.

- [ ] **Step 2: Create build.rs**

```rust
// crates/shepherd-core/build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(
        &["proto/iterm2-api.proto"],
        &["proto/"],
    )?;
    Ok(())
}
```

- [ ] **Step 3: Verify the proto compiles**

```bash
cargo build -p shepherd-core 2>&1 | grep -E "(error|warning.*iterm2|Compiling shepherd-core)" | head -20
```

Expected: `Compiling shepherd-core` with no errors. The generated file will be at `target/debug/build/shepherd-core-*/out/iterm2.rs`.

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-core/proto/ crates/shepherd-core/build.rs
git commit -m "build: vendor iTerm2 proto and add prost_build compile step"
```

---

### Task 2: Add Cargo.toml dependencies

**Files:**
- Modify: `crates/shepherd-core/Cargo.toml`

- [ ] **Step 1: Add runtime dependencies**

In `crates/shepherd-core/Cargo.toml`, add to `[dependencies]`:

```toml
prost = "0.13"
tokio-tungstenite = "0.24"
http = "1.0"
notify = "6"
futures-util = "0.3"
glob = "0.3"
async-trait = "0.1"   # add only if not already present — check first with: grep async-trait Cargo.toml
```

Add a `[build-dependencies]` section:

```toml
[build-dependencies]
prost-build = "0.13"
```

- [ ] **Step 2: Check tokio workspace features**

Open the root `Cargo.toml` (workspace manifest). Find the `[workspace.dependencies]` entry for `tokio`. If it uses `features = ["full"]`, no change is needed — all required features are included. Otherwise ensure it includes at minimum `features = ["net", "time", "fs", "sync", "rt", "macros"]`.

- [ ] **Step 3: Verify clean build**

```bash
cargo build -p shepherd-core 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-core/Cargo.toml Cargo.lock
git commit -m "build(shepherd-core): add prost, tokio-tungstenite, http, notify, futures-util"
```

---

### Task 3: DB migration — iterm2_session_id column

**Files:**
- Modify: `crates/shepherd-core/src/db/mod.rs`
- Modify: `crates/shepherd-core/src/db/models.rs`
- Modify: `crates/shepherd-core/src/db/queries.rs`

- [ ] **Step 1: Write the failing migration test**

At the bottom of `crates/shepherd-core/src/db/mod.rs`, inside `#[cfg(test)] mod tests`, add:

```rust
#[test]
fn test_tasks_table_has_iterm2_session_id_column() {
    let conn = open_memory().unwrap();
    conn.execute(
        "INSERT INTO tasks (title, prompt, agent_id, repo_path, branch, isolation_mode, status, iterm2_session_id)
         VALUES ('t', '', 'claude', '', '', 'none', 'running', 'abc-123')",
        [],
    ).unwrap();
    let val: Option<String> = conn
        .query_row(
            "SELECT iterm2_session_id FROM tasks WHERE title = 't'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(val.as_deref(), Some("abc-123"));
}
```

- [ ] **Step 2: Run test — verify FAIL**

```bash
cargo test -p shepherd-core -- db::tests::test_tasks_table_has_iterm2_session_id_column 2>&1 | tail -8
```

Expected: FAIL with "table tasks has no column named iterm2_session_id".

- [ ] **Step 3: Add the migration**

In `crates/shepherd-core/src/db/mod.rs`, inside `migrate()`, after the line that creates the tasks table, add:

```rust
// Idempotent: silently ignored if column already exists
conn.execute(
    "ALTER TABLE tasks ADD COLUMN iterm2_session_id TEXT",
    [],
).ok();
```

- [ ] **Step 4: Run test — verify PASS**

```bash
cargo test -p shepherd-core -- db::tests::test_tasks_table_has_iterm2_session_id_column 2>&1 | tail -5
```

Expected: PASS.

- [ ] **Step 5: Update Task, CreateTask, TaskEvent — all at once (keep data model coherent)**

**`crates/shepherd-core/src/db/models.rs`** — add to both structs:

```rust
// In Task:
pub iterm2_session_id: Option<String>,

// In CreateTask:
pub iterm2_session_id: Option<String>,
```

**`crates/shepherd-core/src/events.rs`** — add to `TaskEvent`:

```rust
pub iterm2_session_id: Option<String>,
```

Then find every `TaskEvent {` construction site and add `iterm2_session_id: None`:

```bash
grep -rn "TaskEvent {" crates/ --include="*.rs"
```

Update every match.

**`crates/shepherd-core/src/db/queries.rs`** — update every SELECT that fetches tasks. Find them:

```bash
grep -n "SELECT.*FROM tasks" crates/shepherd-core/src/db/queries.rs
```

For each query, add `iterm2_session_id` as the last column:

```sql
SELECT id, title, prompt, agent_id, repo_path, branch, isolation_mode,
       status, created_at, updated_at, iterm2_session_id
FROM tasks ...
```

And in every row-mapping closure/function, add at the end:

```rust
iterm2_session_id: row.get(10)?,
```

If the row-mapping is currently an inline closure duplicated across `get_task` and `list_tasks`, refactor it into a named helper first:

```rust
fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        prompt: row.get(2)?,
        agent_id: row.get(3)?,
        repo_path: row.get(4)?,
        branch: row.get(5)?,
        isolation_mode: row.get(6)?,
        status: TaskStatus::parse_status(&row.get::<_, String>(7)?)
            .unwrap_or(TaskStatus::Queued),
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        iterm2_session_id: row.get(10)?,
    })
}
```

Also update `create_task` INSERT to include `iterm2_session_id` in the column list and bind it from `CreateTask.iterm2_session_id`.

- [ ] **Step 6: Run cargo check to confirm no compile errors after model updates**

```bash
cargo check -p shepherd-core 2>&1 | tail -10
```

Expected: no errors. Fix any `TaskEvent` construction sites that complain about missing `iterm2_session_id`.

- [ ] **Step 7: Write tests for find_task_by_iterm2_id**

In `crates/shepherd-core/src/db/queries.rs`, add to the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open_memory, models::CreateTask};

    fn make_conn() -> rusqlite::Connection {
        open_memory().unwrap()
    }

    #[test]
    fn test_find_task_by_iterm2_id_found() {
        let conn = make_conn();
        let task = create_task(&conn, &CreateTask {
            title: "iterm2 session".into(),
            prompt: None,
            agent_id: "claude".into(),
            repo_path: None,
            isolation_mode: None,
            iterm2_session_id: Some("session-xyz".into()),
        }).unwrap();
        let found = find_task_by_iterm2_id(&conn, "session-xyz").unwrap();
        assert_eq!(found.map(|t| t.id), Some(task.id));
    }

    #[test]
    fn test_find_task_by_iterm2_id_not_found() {
        let conn = make_conn();
        let result = find_task_by_iterm2_id(&conn, "no-such-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update_task_status_changes_status() {
        use crate::db::models::TaskStatus;
        let conn = make_conn();
        let task = create_task(&conn, &CreateTask {
            title: "status test".into(),
            prompt: None,
            agent_id: "claude".into(),
            repo_path: None,
            isolation_mode: None,
            iterm2_session_id: None,
        }).unwrap();
        update_task_status(&conn, task.id, &TaskStatus::Done).unwrap();
        let updated = get_task(&conn, task.id).unwrap().unwrap();
        assert_eq!(updated.status, TaskStatus::Done);
    }
}
```

- [ ] **Step 8: Implement find_task_by_iterm2_id and update_task_status**

Add to `crates/shepherd-core/src/db/queries.rs`:

```rust
pub fn find_task_by_iterm2_id(
    conn: &rusqlite::Connection,
    iterm2_id: &str,
) -> rusqlite::Result<Option<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, prompt, agent_id, repo_path, branch, isolation_mode,
                status, created_at, updated_at, iterm2_session_id
         FROM tasks WHERE iterm2_session_id = ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query_map([iterm2_id], row_to_task)?;
    rows.next().transpose()
}

pub fn update_task_status(
    conn: &rusqlite::Connection,
    task_id: i64,
    status: &crate::db::models::TaskStatus,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tasks SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![status.as_str(), task_id],
    )?;
    Ok(())
}
```

Note: `row_to_task` must be a shared helper closure or function — look at how existing `get_task` / `list_tasks` map rows and extract it into a reusable function if it isn't already.

- [ ] **Step 9: Run all DB tests**

```bash
cargo test -p shepherd-core -- db:: 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 10: Commit**

```bash
git add crates/shepherd-core/src/db/ crates/shepherd-core/src/events.rs
git commit -m "feat(db): add iterm2_session_id column; find_task_by_iterm2_id, update_task_status, TaskEvent field"
```

---

## Chunk 2: Core iterm2 Module

### Task 4: Auth module

**Files:**
- Create: `crates/shepherd-core/src/iterm2/auth.rs`
- Create: `crates/shepherd-core/src/iterm2/mod.rs` (stub)

- [ ] **Step 1: Create the module stub**

```rust
// crates/shepherd-core/src/iterm2/mod.rs
pub mod auth;
pub mod client;
pub mod scanner;
pub mod session;
```

Add to `crates/shepherd-core/src/lib.rs`:
```rust
pub mod iterm2;
```

- [ ] **Step 2: Write auth tests first**

```rust
// crates/shepherd-core/src/iterm2/auth.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_auth_ok() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"cookie":"cook1","key":"key1"}}"#).unwrap();
        let auth = load_auth(f.path()).unwrap();
        assert_eq!(auth.cookie, "cook1");
        assert_eq!(auth.key, "key1");
    }

    #[test]
    fn test_load_auth_missing_file() {
        let result = load_auth(std::path::Path::new("/nonexistent/iterm2-auth.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_auth_invalid_json() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "not json").unwrap();
        let result = load_auth(f.path());
        assert!(result.is_err());
    }
}
```

- [ ] **Step 3: Run tests — verify FAIL**

```bash
cargo test -p shepherd-core -- iterm2::auth::tests 2>&1 | tail -8
```

Expected: compile error (module does not exist yet).

- [ ] **Step 4: Implement auth.rs**

```rust
// crates/shepherd-core/src/iterm2/auth.rs
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iterm2Auth {
    pub cookie: String,
    pub key: String,
}

/// Read auth credentials from the JSON file written by the AutoLaunch bridge script.
pub fn load_auth(path: &Path) -> anyhow::Result<Iterm2Auth> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("reading iTerm2 auth from {}", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| "parsing iTerm2 auth JSON")
}

/// Default path where the AutoLaunch bridge script writes credentials.
pub fn default_auth_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".shepherd").join("iterm2-auth.json")
}

/// Spawn a background thread that watches the auth file for changes and sends
/// the new credentials over `tx`. Uses std::sync::mpsc to bridge the blocking
/// notify callback into the async world.
pub fn watch_auth(path: PathBuf, tx: mpsc::Sender<Iterm2Auth>) {
    std::thread::spawn(move || {
        use notify::{Event, RecursiveMode, Watcher};

        let (sync_tx, sync_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = match notify::RecommendedWatcher::new(
            move |res| { let _ = sync_tx.send(res); },
            notify::Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("iterm2 auth watcher init failed: {e}");
                return;
            }
        };
        if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
            tracing::warn!("iterm2 auth watcher watch failed: {e}");
            return;
        }
        for event in sync_rx {
            match event {
                Ok(_) => {
                    match load_auth(&path) {
                        Ok(auth) => {
                            // blocking_send is fine — we're on a dedicated std thread
                            let _ = tx.blocking_send(auth);
                        }
                        Err(e) => tracing::warn!("iterm2 auth reload failed: {e}"),
                    }
                }
                Err(e) => tracing::warn!("iterm2 auth watch event error: {e}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_auth_ok() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"cookie":"cook1","key":"key1"}}"#).unwrap();
        let auth = load_auth(f.path()).unwrap();
        assert_eq!(auth.cookie, "cook1");
        assert_eq!(auth.key, "key1");
    }

    #[test]
    fn test_load_auth_missing_file() {
        let result = load_auth(std::path::Path::new("/nonexistent/iterm2-auth.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_auth_invalid_json() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "not json").unwrap();
        let result = load_auth(f.path());
        assert!(result.is_err());
    }
}
```

Add `tempfile = "3"` to `[dev-dependencies]` in `crates/shepherd-core/Cargo.toml`.

- [ ] **Step 5: Run tests — verify PASS**

```bash
cargo test -p shepherd-core -- iterm2::auth::tests 2>&1 | tail -8
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/iterm2/ crates/shepherd-core/src/lib.rs crates/shepherd-core/Cargo.toml
git commit -m "feat(iterm2): auth module — load and watch ~/.shepherd/iterm2-auth.json"
```

---

### Task 5: Transport trait and WebSocket client

**Files:**
- Create: `crates/shepherd-core/src/iterm2/client.rs`

The `Iterm2Transport` trait makes the scanner and session modules testable without a real iTerm2 connection. Tests inject a `MockTransport`; production code uses `WsClient`.

- [ ] **Step 1: Write transport trait tests first**

```rust
// crates/shepherd-core/src/iterm2/client.rs
#[cfg(test)]
mod tests {
    use super::*;

    // A mock transport that replays pre-canned response bytes
    struct MockTransport {
        // Queue of (expected_request_id, response_bytes) pairs
        responses: std::collections::VecDeque<Vec<u8>>,
    }

    #[async_trait::async_trait]
    impl Iterm2Transport for MockTransport {
        async fn send_recv(
            &mut self,
            msg: iterm2::ClientOriginatedMessage,
        ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
            let _ = msg; // discard
            let bytes = self.responses.pop_front()
                .ok_or_else(|| anyhow::anyhow!("MockTransport: no more responses"))?;
            Ok(prost::Message::decode(bytes.as_slice())?)
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        use prost::Message;
        // Build a minimal ClientOriginatedMessage with a ListSessionsRequest
        let req = iterm2::ClientOriginatedMessage {
            id: Some(42),
            list_sessions_request: Some(iterm2::ListSessionsRequest {}),
            ..Default::default()
        };
        let encoded = req.encode_to_vec();
        let decoded: iterm2::ClientOriginatedMessage =
            prost::Message::decode(encoded.as_slice()).unwrap();
        assert_eq!(decoded.id, Some(42));
        assert!(decoded.list_sessions_request.is_some());
    }
}
```

- [ ] **Step 2: Run test — verify it fails to compile (module missing)**

```bash
cargo test -p shepherd-core -- iterm2::client::tests 2>&1 | tail -8
```

- [ ] **Step 3: Implement client.rs**

```rust
// crates/shepherd-core/src/iterm2/client.rs
use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio_tungstenite::tungstenite::Message;

// Include prost-generated types. The file is named after the proto package ("iterm2").
#[allow(clippy::all)]
pub mod iterm2 {
    include!(concat!(env!("OUT_DIR"), "/iterm2.rs"));
}

static MSG_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    MSG_ID.fetch_add(1, Ordering::Relaxed)
}

/// Abstraction over the WebSocket connection to iTerm2.
/// Implemented by WsClient (production) and MockTransport (tests).
#[async_trait::async_trait]
pub trait Iterm2Transport: Send {
    async fn send_recv(
        &mut self,
        msg: iterm2::ClientOriginatedMessage,
    ) -> anyhow::Result<iterm2::ServerOriginatedMessage>;

    /// Send a message without waiting for a response (for subscriptions and
    /// fire-and-forget requests like SendTextRequest).
    async fn send_only(
        &mut self,
        msg: iterm2::ClientOriginatedMessage,
    ) -> anyhow::Result<()>;

    /// Receive the next unsolicited server message (notifications).
    async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage>;
}

/// Production WebSocket client connecting over a Unix domain socket.
pub struct WsClient {
    ws: tokio_tungstenite::WebSocketStream<tokio::net::UnixStream>,
}

impl WsClient {
    /// Connect to iTerm2 over the Unix domain socket.
    /// `socket_path` is discovered by globbing
    /// `~/Library/Application Support/iTerm2/iterm2-daemon-*.socket`.
    pub async fn connect(
        socket_path: &std::path::Path,
        auth: &crate::iterm2::auth::Iterm2Auth,
    ) -> anyhow::Result<Self> {
        let stream = tokio::net::UnixStream::connect(socket_path)
            .await
            .with_context(|| format!("connecting to iTerm2 socket {}", socket_path.display()))?;

        let req = http::Request::builder()
            .uri("ws://localhost/")
            .header("x-iterm2-cookie", &auth.cookie)
            .header("x-iterm2-key", &auth.key)
            .body(())
            .context("building WebSocket handshake request")?;

        let (ws, _) = tokio_tungstenite::client_async_with_config(req, stream, None)
            .await
            .context("WebSocket handshake with iTerm2")?;

        Ok(Self { ws })
    }
}

#[async_trait::async_trait]
impl Iterm2Transport for WsClient {
    async fn send_recv(
        &mut self,
        mut msg: iterm2::ClientOriginatedMessage,
    ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
        msg.id = Some(next_id());
        let bytes = msg.encode_to_vec();
        self.ws.send(Message::Binary(bytes)).await
            .context("sending iTerm2 request")?;

        // Read frames until we get a response with matching id
        loop {
            let frame = self.ws.next().await
                .context("iTerm2 connection closed")?
                .context("iTerm2 WebSocket error")?;
            if let Message::Binary(payload) = frame {
                let resp: iterm2::ServerOriginatedMessage =
                    ProstMessage::decode(payload.as_slice())
                        .context("decoding iTerm2 ServerOriginatedMessage")?;
                return Ok(resp);
            }
        }
    }

    async fn send_only(
        &mut self,
        mut msg: iterm2::ClientOriginatedMessage,
    ) -> anyhow::Result<()> {
        msg.id = Some(next_id());
        let bytes = msg.encode_to_vec();
        self.ws.send(Message::Binary(bytes)).await
            .context("sending iTerm2 fire-and-forget message")?;
        Ok(())
    }

    async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
        loop {
            let frame = self.ws.next().await
                .context("iTerm2 connection closed")?
                .context("iTerm2 WebSocket error")?;
            if let Message::Binary(payload) = frame {
                return ProstMessage::decode(payload.as_slice())
                    .context("decoding iTerm2 notification");
            }
        }
    }
}

/// Discover the iTerm2 Unix socket path by globbing.
/// Returns `Err` if iTerm2 is not running.
pub fn find_socket() -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let pattern = format!(
        "{}/Library/Application Support/iTerm2/iterm2-daemon-*.socket",
        home
    );
    let mut matches: Vec<_> = glob::glob(&pattern)
        .context("globbing iTerm2 socket")?
        .filter_map(Result::ok)
        .collect();
    matches.sort();
    matches.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("iTerm2 socket not found — is iTerm2 running?"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let req = iterm2::ClientOriginatedMessage {
            id: Some(42),
            list_sessions_request: Some(iterm2::ListSessionsRequest {}),
            ..Default::default()
        };
        let encoded = req.encode_to_vec();
        let decoded: iterm2::ClientOriginatedMessage =
            ProstMessage::decode(encoded.as_slice()).unwrap();
        assert_eq!(decoded.id, Some(42));
        assert!(decoded.list_sessions_request.is_some());
    }
}
```

Both `glob` and `async-trait` were already added in Task 2.

- [ ] **Step 4: Run tests — verify PASS**

```bash
cargo test -p shepherd-core -- iterm2::client::tests 2>&1 | tail -8
```

Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/iterm2/client.rs crates/shepherd-core/Cargo.toml
git commit -m "feat(iterm2): Iterm2Transport trait + WsClient over UnixStream"
```

---

### Task 6: Scanner module

**Files:**
- Create: `crates/shepherd-core/src/iterm2/scanner.rs`

- [ ] **Step 1: Write scanner tests**

```rust
// At bottom of crates/shepherd-core/src/iterm2/scanner.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::iterm2::client::iterm2;

    fn make_list_sessions_response(sessions: Vec<(&str, &str)>) -> iterm2::ServerOriginatedMessage {
        // sessions: Vec<(unique_identifier, title)>
        let session_summaries: Vec<iterm2::SessionSummary> = sessions
            .into_iter()
            .map(|(id, title)| iterm2::SessionSummary {
                unique_identifier: Some(id.to_string()),
                title: Some(title.to_string()),
                ..Default::default()
            })
            .collect();
        let leaf = iterm2::SplitTreeNode {
            session_summary: Some(session_summaries[0].clone()),
            ..Default::default()
        };
        let tab = iterm2::Tab {
            root: Some(iterm2::SplitTreeNode {
                children: vec![leaf],
                session_summary: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let window = iterm2::Window {
            tabs: vec![tab],
            ..Default::default()
        };
        iterm2::ServerOriginatedMessage {
            list_sessions_response: Some(iterm2::ListSessionsResponse {
                windows: vec![window],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn make_variable_response(value: &str) -> iterm2::ServerOriginatedMessage {
        iterm2::ServerOriginatedMessage {
            variable_response: Some(iterm2::VariableResponse {
                value: Some(value.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_scan_finds_claude_session() {
        // A mock transport that returns:
        // 1. ListSessionsResponse with one session
        // 2. VariableResponse("claude") for jobName
        // 3. VariableResponse("/home/user/myproject") for path
        struct MockT {
            calls: usize,
        }
        #[async_trait::async_trait]
        impl crate::iterm2::client::Iterm2Transport for MockT {
            async fn send_recv(
                &mut self,
                _msg: iterm2::ClientOriginatedMessage,
            ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                let r = match self.calls {
                    0 => make_list_sessions_response(vec![("sess-1", "bash")]),
                    1 => make_variable_response("claude"),
                    2 => make_variable_response("/home/user/myproject"),
                    _ => panic!("unexpected call"),
                };
                self.calls += 1;
                Ok(r)
            }
            async fn send_only(
                &mut self,
                _msg: iterm2::ClientOriginatedMessage,
            ) -> anyhow::Result<()> { Ok(()) }
            async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                futures_util::future::pending().await
            }
        }

        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockT { calls: 0 }).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].iterm2_session_id, "sess-1");
        assert_eq!(candidates[0].cwd, "/home/user/myproject");
    }

    #[tokio::test]
    async fn test_scan_skips_non_claude_session() {
        struct MockT;
        #[async_trait::async_trait]
        impl crate::iterm2::client::Iterm2Transport for MockT {
            async fn send_recv(
                &mut self,
                _msg: iterm2::ClientOriginatedMessage,
            ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                Ok(make_list_sessions_response(vec![("sess-2", "vim")]))
            }
            async fn send_only(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<()> { Ok(()) }
            async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                futures_util::future::pending().await
            }
        }
        // After ListSessions, no VariableRequest is made because there are no sessions
        // Hmm — but the scanner always sends VariableRequest(jobName). Let's make
        // the mock return "vim" for jobName.
        struct MockT2 { calls: usize }
        #[async_trait::async_trait]
        impl crate::iterm2::client::Iterm2Transport for MockT2 {
            async fn send_recv(
                &mut self,
                _: iterm2::ClientOriginatedMessage,
            ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                let r = match self.calls {
                    0 => make_list_sessions_response(vec![("sess-2", "vim")]),
                    1 => make_variable_response("vim"),   // not "claude"
                    _ => panic!("unexpected"),
                };
                self.calls += 1;
                Ok(r)
            }
            async fn send_only(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<()> { Ok(()) }
            async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                futures_util::future::pending().await
            }
        }
        let mut scanner = Scanner::new(std::collections::HashSet::new());
        let candidates = scanner.scan(&mut MockT2 { calls: 0 }).await.unwrap();
        assert!(candidates.is_empty());
    }

    #[tokio::test]
    async fn test_scan_deduplicates_already_adopted() {
        struct MockT;
        #[async_trait::async_trait]
        impl crate::iterm2::client::Iterm2Transport for MockT {
            async fn send_recv(
                &mut self,
                _: iterm2::ClientOriginatedMessage,
            ) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                // Only call: ListSessions — session already in adopted set, so no VariableRequest
                Ok(make_list_sessions_response(vec![("sess-3", "claude")]))
            }
            async fn send_only(&mut self, _: iterm2::ClientOriginatedMessage) -> anyhow::Result<()> { Ok(()) }
            async fn recv(&mut self) -> anyhow::Result<iterm2::ServerOriginatedMessage> {
                futures_util::future::pending().await
            }
        }
        let mut adopted = std::collections::HashSet::new();
        adopted.insert("sess-3".to_string());
        let mut scanner = Scanner::new(adopted);
        let candidates = scanner.scan(&mut MockT).await.unwrap();
        assert!(candidates.is_empty());
    }
}
```

- [ ] **Step 2: Run tests — verify compile failure**

```bash
cargo test -p shepherd-core -- iterm2::scanner::tests 2>&1 | tail -5
```

- [ ] **Step 3: Implement scanner.rs**

```rust
// crates/shepherd-core/src/iterm2/scanner.rs
use crate::iterm2::client::{iterm2, Iterm2Transport};
use std::collections::HashSet;

#[derive(Debug)]
pub struct AdoptionCandidate {
    pub iterm2_session_id: String,
    pub cwd: String,
}

pub struct Scanner {
    adopted: HashSet<String>,
}

impl Scanner {
    pub fn new(adopted: HashSet<String>) -> Self {
        Self { adopted }
    }

    pub fn mark_adopted(&mut self, session_id: String) {
        self.adopted.insert(session_id);
    }

    pub fn mark_terminated(&mut self, session_id: &str) {
        self.adopted.remove(session_id);
    }

    /// One scan pass: list sessions, query jobName for unadopted ones,
    /// return candidates where jobName contains "claude".
    pub async fn scan(
        &mut self,
        transport: &mut dyn Iterm2Transport,
    ) -> anyhow::Result<Vec<AdoptionCandidate>> {
        // 1. List all sessions
        let resp = transport.send_recv(iterm2::ClientOriginatedMessage {
            list_sessions_request: Some(iterm2::ListSessionsRequest {}),
            ..Default::default()
        }).await?;

        let list_resp = resp.list_sessions_response
            .ok_or_else(|| anyhow::anyhow!("expected ListSessionsResponse"))?;

        // 2. Walk windows → tabs → SplitTreeNode tree
        let session_ids: Vec<String> = collect_session_ids(&list_resp.windows);

        // 3. For each unadopted session, query jobName
        let mut candidates = Vec::new();
        for session_id in session_ids {
            if self.adopted.contains(&session_id) {
                continue;
            }
            // Query jobName
            let job_resp = transport.send_recv(iterm2::ClientOriginatedMessage {
                variable_request: Some(iterm2::VariableRequest {
                    session: Some(session_id.clone()),
                    name: "jobName".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }).await?;

            let job_name = job_resp
                .variable_response
                .and_then(|r| r.value)
                .unwrap_or_default();

            if !job_name.contains("claude") {
                continue;
            }

            // Query CWD (path variable, requires shell integration)
            let path_resp = transport.send_recv(iterm2::ClientOriginatedMessage {
                variable_request: Some(iterm2::VariableRequest {
                    session: Some(session_id.clone()),
                    name: "path".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }).await?;

            let cwd = path_resp
                .variable_response
                .and_then(|r| r.value)
                .unwrap_or_default();

            // Subscribe to screen updates for this session
            transport.send_only(iterm2::ClientOriginatedMessage {
                notification_request: Some(iterm2::NotificationRequest {
                    session: Some(session_id.clone()),
                    subscribe: Some(true),
                    notification_type: Some(
                        iterm2::notification_type::NotificationType::NotifyOnScreenUpdate as i32,
                    ),
                    ..Default::default()
                }),
                ..Default::default()
            }).await?;

            candidates.push(AdoptionCandidate { iterm2_session_id: session_id, cwd });
        }
        Ok(candidates)
    }

    /// Subscribe globally to session termination (must be called once after first adoption).
    pub async fn subscribe_terminate(&self, transport: &mut dyn Iterm2Transport) -> anyhow::Result<()> {
        transport.send_only(iterm2::ClientOriginatedMessage {
            notification_request: Some(iterm2::NotificationRequest {
                subscribe: Some(true),
                notification_type: Some(
                    iterm2::notification_type::NotificationType::NotifyOnTerminateSession as i32,
                ),
                ..Default::default()
            }),
            ..Default::default()
        }).await
    }
}

/// Recursively walk the SplitTreeNode tree to collect all session unique_identifiers.
fn collect_session_ids(windows: &[iterm2::Window]) -> Vec<String> {
    let mut ids = Vec::new();
    for window in windows {
        for tab in &window.tabs {
            if let Some(root) = &tab.root {
                walk_node(root, &mut ids);
            }
        }
    }
    ids
}

fn walk_node(node: &iterm2::SplitTreeNode, out: &mut Vec<String>) {
    if let Some(summary) = &node.session_summary {
        if let Some(id) = &summary.unique_identifier {
            out.push(id.clone());
        }
    }
    for child in &node.children {
        walk_node(child, out);
    }
}
```

Note: `iterm2::notification_type::NotificationType` path depends on how prost generates the enum. Check the generated `iterm2.rs` in `target/.../out/iterm2.rs` and adjust the path if needed. The value for `NOTIFY_ON_SCREEN_UPDATE` is defined in the proto as an enum variant — use the exact generated constant name.

- [ ] **Step 4: Run tests — verify PASS**

```bash
cargo test -p shepherd-core -- iterm2::scanner::tests 2>&1 | tail -10
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/iterm2/scanner.rs
git commit -m "feat(iterm2): scanner — discover claude sessions, emit AdoptionCandidate"
```

---

### Task 7: Session module — cell flattening and gate detection

**Files:**
- Create: `crates/shepherd-core/src/iterm2/session.rs`

- [ ] **Step 1: Write cell flattening and gate detection tests**

```rust
// In crates/shepherd-core/src/iterm2/session.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;
    use crate::iterm2::client::iterm2;

    fn make_line(cells: &[&str], hard_eol: bool) -> iterm2::LineContents {
        iterm2::LineContents {
            cell: cells.iter().map(|s| iterm2::Cell {
                string_value: Some(s.to_string()),
                ..Default::default()
            }).collect(),
            hard_eol: Some(hard_eol),
            ..Default::default()
        }
    }

    #[test]
    fn test_flatten_hard_eol_appends_newline() {
        let lines = vec![
            make_line(&["h", "e", "l", "l", "o"], true),
            make_line(&["w", "o", "r", "l", "d"], true),
        ];
        let text = flatten_buffer(&lines);
        assert_eq!(text, "hello\nworld\n");
    }

    #[test]
    fn test_flatten_soft_wrap_no_newline() {
        let lines = vec![
            make_line(&["a", "b", "c"], false), // soft-wrap: continue
            make_line(&["d", "e", "f"], true),  // hard eol: newline here
        ];
        let text = flatten_buffer(&lines);
        assert_eq!(text, "abcdef\n");
    }

    #[test]
    fn test_flatten_empty_buffer() {
        let text = flatten_buffer(&[]);
        assert_eq!(text, "");
    }

    #[test]
    fn test_detect_permission_prompt_bash_tool() {
        let text = "Allow bash tool?\n(y/n) [y]: ";
        assert!(detect_permission_prompt(text).is_some());
    }

    #[test]
    fn test_detect_permission_prompt_write_tool() {
        let text = "Allow write to file?\n(y/n): ";
        assert!(detect_permission_prompt(text).is_some());
    }

    #[test]
    fn test_detect_permission_prompt_no_match() {
        let text = "Normal terminal output with no prompt";
        assert!(detect_permission_prompt(text).is_none());
    }

    #[test]
    fn test_detect_permission_prompt_extracts_tool_name() {
        let text = "Allow bash tool?\n(y/n) [y]: ";
        let tool = detect_permission_prompt(text).unwrap();
        assert!(tool.contains("bash"), "tool name should contain 'bash', got: {tool}");
    }
}
```

- [ ] **Step 2: Run tests — verify FAIL**

```bash
cargo test -p shepherd-core -- iterm2::session::tests 2>&1 | tail -5
```

- [ ] **Step 3: Implement flatten_buffer and detect_permission_prompt**

```rust
// crates/shepherd-core/src/iterm2/session.rs
use crate::iterm2::client::iterm2;

/// Flatten a GetBufferResponse line list to plain text.
/// Appends '\n' only on hard_eol lines; soft-wrap lines are concatenated without separator.
pub fn flatten_buffer(lines: &[iterm2::LineContents]) -> String {
    let mut out = String::new();
    for line in lines {
        for cell in &line.cell {
            if let Some(ref s) = cell.string_value {
                out.push_str(s);
            }
        }
        if line.hard_eol.unwrap_or(false) {
            out.push('\n');
        }
    }
    out
}

/// Detect a Claude Code permission prompt in terminal output.
/// Claude Code prompts look like:  "Allow <tool> tool?\n(y/n)"
/// Returns the extracted tool description if found.
pub fn detect_permission_prompt(text: &str) -> Option<String> {
    // Match patterns like "Allow X?\n(y/n)" or "Allow X? (y/n)"
    let lower = text.to_lowercase();
    if lower.contains("(y/n)") {
        // Find the "Allow ... ?" portion preceding "(y/n)"
        if let Some(allow_pos) = lower.rfind("allow ") {
            let fragment = &text[allow_pos..];
            if let Some(q_pos) = fragment.find('?') {
                let tool_desc = fragment[6..q_pos].trim().to_string();
                return Some(tool_desc);
            }
        }
    }
    None
}

/// Represents an adopted iTerm2 session actively managed by Shepherd.
pub struct AdoptedSession {
    pub task_id: i64,
    pub iterm2_session_id: String,
    pub cwd: String,
}

impl AdoptedSession {
    pub fn new(task_id: i64, iterm2_session_id: String, cwd: String) -> Self {
        Self { task_id, iterm2_session_id, cwd }
    }
}
```

- [ ] **Step 4: Run tests — verify PASS**

```bash
cargo test -p shepherd-core -- iterm2::session::tests 2>&1 | tail -10
```

Expected: 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/iterm2/session.rs
git commit -m "feat(iterm2): session — cell flattening with hard_eol, permission prompt detection"
```

---

### Task 8: Iterm2Manager — public API and background loop

**Files:**
- Modify: `crates/shepherd-core/src/iterm2/mod.rs`

- [ ] **Step 1: Write manager tests**

```rust
// In iterm2/mod.rs tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_claude_sessions_empty_dir() {
        // Non-existent projects dir returns empty list
        let sessions = list_claude_sessions("/nonexistent/path");
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_list_claude_sessions_lists_jsonl_files() {
        let dir = tempfile::tempdir().unwrap();
        // Create some fake .jsonl files with timestamps
        std::fs::write(dir.path().join("abc-111.jsonl"), "{}").unwrap();
        std::fs::write(dir.path().join("abc-222.jsonl"), "{}").unwrap();
        let sessions = list_claude_sessions(dir.path().to_str().unwrap());
        assert_eq!(sessions.len(), 2);
        // IDs are the stem (filename without extension)
        assert!(sessions.iter().any(|s| s == "abc-111" || s == "abc-222"));
    }

    #[test]
    fn test_encode_cwd_for_path() {
        // Claude encodes the CWD using percent-encoding with / → -
        // "/home/user/myproject" → "-home-user-myproject"
        let encoded = encode_cwd_for_projects("/home/user/myproject");
        assert_eq!(encoded, "-home-user-myproject");
    }
}
```

- [ ] **Step 2: Run tests — verify FAIL**

```bash
cargo test -p shepherd-core -- iterm2::tests 2>&1 | tail -5
```

- [ ] **Step 3: Implement mod.rs — public API**

```rust
// crates/shepherd-core/src/iterm2/mod.rs
pub mod auth;
pub mod client;
pub mod scanner;
pub mod session;

use anyhow::Context;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::db;
use crate::db::models::{CreateTask, TaskStatus};
use crate::events::ServerEvent;

/// Encode a CWD path the same way Claude Code does when naming project directories.
/// Claude uses the path with '/' replaced by '-' (and a leading '-' for absolute paths).
pub fn encode_cwd_for_projects(cwd: &str) -> String {
    cwd.replace('/', "-")
}

/// List Claude Code session IDs available for resume in a given CWD.
/// Returns session IDs (JSONL filename stems) sorted newest-first by mtime.
pub fn list_claude_sessions(project_dir: &str) -> Vec<String> {
    let dir = Path::new(project_dir);
    if !dir.exists() {
        return vec![];
    }
    let mut entries: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |x| x == "jsonl"))
                .filter_map(|e| {
                    let mtime = e.metadata().ok()?.modified().ok()?;
                    let stem = e.path().file_stem()?.to_str()?.to_string();
                    Some((mtime, stem))
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.into_iter().map(|(_, stem)| stem).collect()
}

/// Resolve the Claude projects directory for a given CWD.
pub fn claude_project_dir(cwd: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let encoded = encode_cwd_for_projects(cwd);
    PathBuf::from(home).join(".claude").join("projects").join(encoded)
}

/// Handle for the running iTerm2 integration.
pub struct Iterm2Manager {
    /// Map of iTerm2 session ID → adopted session state (task_id, cwd)
    adopted: Arc<Mutex<std::collections::HashMap<String, session::AdoptedSession>>>,
    auth_path: PathBuf,
}

impl Iterm2Manager {
    pub fn new(auth_path: PathBuf) -> Self {
        Self {
            adopted: Arc::new(Mutex::new(std::collections::HashMap::new())),
            auth_path,
        }
    }

    /// Check whether the auth credentials file exists.
    pub fn is_auth_configured(&self) -> bool {
        self.auth_path.exists()
    }

    pub async fn get_adopted_cwd(&self, iterm2_session_id: &str) -> Option<String> {
        let guard = self.adopted.lock().await;
        guard.get(iterm2_session_id).map(|s| s.cwd.clone())
    }

    pub async fn get_task_id_for_iterm2(&self, iterm2_session_id: &str) -> Option<i64> {
        let guard = self.adopted.lock().await;
        guard.get(iterm2_session_id).map(|s| s.task_id)
    }

    /// Spawn the background adoption loop. Should be called once at startup.
    /// The loop: loads auth → connects → polls every 5 s → adopts new sessions.
    pub fn spawn(
        self: Arc<Self>,
        db: Arc<Mutex<rusqlite::Connection>>,
        event_tx: broadcast::Sender<ServerEvent>,
    ) {
        tokio::spawn(async move {
            self.run_loop(db, event_tx).await;
        });
    }

    async fn run_loop(
        &self,
        db: Arc<Mutex<rusqlite::Connection>>,
        event_tx: broadcast::Sender<ServerEvent>,
    ) {
        loop {
            match auth::load_auth(&self.auth_path) {
                Err(e) => {
                    tracing::debug!("iTerm2 auth not available ({e}), skipping scan");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
                Ok(auth) => {
                    if let Err(e) = self.run_connected(&auth, &db, &event_tx).await {
                        tracing::warn!("iTerm2 session loop error: {e:#}");
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn run_connected(
        &self,
        auth: &auth::Iterm2Auth,
        db: &Arc<Mutex<rusqlite::Connection>>,
        event_tx: &broadcast::Sender<ServerEvent>,
    ) -> anyhow::Result<()> {
        let socket = client::find_socket()?;
        let mut ws = client::WsClient::connect(&socket, auth).await?;

        let adopted_ids: std::collections::HashSet<String> = {
            self.adopted.lock().await.keys().cloned().collect()
        };
        let mut scanner = scanner::Scanner::new(adopted_ids);
        let candidates = scanner.scan(&mut ws).await?;

        if !candidates.is_empty() {
            scanner.subscribe_terminate(&mut ws).await?;
        }

        for candidate in candidates {
            let task = {
                let conn = db.lock().await;
                crate::db::queries::create_task(&conn, &CreateTask {
                    title: format!("iTerm2: {}", candidate.cwd),
                    prompt: None,
                    agent_id: "iterm2-adopted".to_string(),
                    repo_path: Some(candidate.cwd.clone()),
                    isolation_mode: Some("none".to_string()),
                    iterm2_session_id: Some(candidate.iterm2_session_id.clone()),
                })?
            };
            tracing::info!("Adopted iTerm2 session {} as task {}", candidate.iterm2_session_id, task.id);
            let _ = event_tx.send(ServerEvent::TaskCreated {
                task: crate::events::TaskEvent {
                    id: task.id,
                    title: task.title.clone(),
                    agent_id: task.agent_id.clone(),
                    status: task.status.clone(),
                    branch: task.branch.clone(),
                    repo_path: task.repo_path.clone(),
                    iterm2_session_id: Some(candidate.iterm2_session_id.clone()),
                },
            });
            let mut guard = self.adopted.lock().await;
            guard.insert(
                candidate.iterm2_session_id.clone(),
                session::AdoptedSession::new(task.id, candidate.iterm2_session_id, candidate.cwd),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_claude_sessions_empty_dir() {
        let sessions = list_claude_sessions("/nonexistent/path");
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_list_claude_sessions_lists_jsonl_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("abc-111.jsonl"), "{}").unwrap();
        std::fs::write(dir.path().join("abc-222.jsonl"), "{}").unwrap();
        let sessions = list_claude_sessions(dir.path().to_str().unwrap());
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_encode_cwd_for_path() {
        let encoded = encode_cwd_for_projects("/home/user/myproject");
        assert_eq!(encoded, "-home-user-myproject");
    }
}
```

Note: `ServerEvent::TaskCreated { task: TaskEvent { ... iterm2_session_id } }` — you will need to add `iterm2_session_id: Option<String>` to `TaskEvent` in `crates/shepherd-core/src/events.rs`.

- [ ] **Step 4: Update events.rs to add iterm2_session_id**

In `crates/shepherd-core/src/events.rs`, add `iterm2_session_id: Option<String>` to `TaskEvent`. Update all `TaskEvent { ... }` construction sites across the codebase to include `iterm2_session_id: None` (existing non-iTerm2 tasks).

- [ ] **Step 5: Run all iterm2 tests**

```bash
cargo test -p shepherd-core -- iterm2:: 2>&1 | tail -15
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/iterm2/ crates/shepherd-core/src/events.rs
git commit -m "feat(iterm2): Iterm2Manager — adoption loop, session discovery, task creation"
```

---

## Chunk 3: Server Integration

### Task 9: Extend AppState and register routes

**Files:**
- Modify: `crates/shepherd-server/src/state.rs`
- Modify: `crates/shepherd-server/src/lib.rs`
- Modify: `crates/shepherd-server/src/routes/mod.rs`
- Create: `crates/shepherd-server/src/routes/iterm2.rs`

- [ ] **Step 1: Add iterm2 to AppState**

Open `crates/shepherd-server/src/state.rs`. Add the import and the field:

```rust
use shepherd_core::iterm2::Iterm2Manager;
use std::sync::Arc;

pub struct AppState {
    // ... existing fields ...
    pub iterm2: Option<Arc<Iterm2Manager>>,
}
```

- [ ] **Step 2: Write failing tests for the new routes**

Create `crates/shepherd-server/src/routes/iterm2.rs` with tests first:

```rust
// crates/shepherd-server/src/routes/iterm2.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Serialize)]
pub struct ClaudeSessionsResponse {
    pub sessions: Vec<String>,
}

#[derive(Deserialize)]
pub struct ResumeRequest {
    pub claude_session_id: Option<String>,
}

pub async fn list_claude_sessions(
    Path(task_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ClaudeSessionsResponse>, StatusCode> {
    // Look up task → get repo_path (CWD) → list JSONL files
    let cwd = {
        let conn = state.db.lock().await;
        let task = shepherd_core::db::queries::get_task(&conn, task_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
        task.repo_path
    };

    let project_dir = shepherd_core::iterm2::claude_project_dir(&cwd);
    let sessions = shepherd_core::iterm2::list_claude_sessions(
        project_dir.to_str().unwrap_or(""),
    );

    Ok(Json(ClaudeSessionsResponse { sessions }))
}

pub async fn resume_session(
    Path(task_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<ResumeRequest>,
) -> StatusCode {
    // Look up iterm2_session_id for this task, then send resume command
    let iterm2_session_id = {
        let conn = state.db.lock().await;
        match shepherd_core::db::queries::get_task(&conn, task_id) {
            Ok(Some(t)) => t.iterm2_session_id,
            _ => return StatusCode::NOT_FOUND,
        }
    };

    let Some(session_id) = iterm2_session_id else {
        return StatusCode::BAD_REQUEST; // not an iTerm2 task
    };

    // The actual resume mechanism (Ctrl-C + relaunch) is handled by the
    // Iterm2Manager background loop which has the live WebSocket connection.
    // Here we just log the intent; the full resume flow requires the manager.
    tracing::info!(
        "Resume requested for task {task_id} (session {session_id}) with claude_session_id={:?}",
        body.claude_session_id
    );

    StatusCode::ACCEPTED
}

pub async fn fresh_session(
    Path(task_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    let iterm2_session_id = {
        let conn = state.db.lock().await;
        match shepherd_core::db::queries::get_task(&conn, task_id) {
            Ok(Some(t)) => t.iterm2_session_id,
            _ => return StatusCode::NOT_FOUND,
        }
    };

    if iterm2_session_id.is_none() {
        return StatusCode::BAD_REQUEST;
    }

    StatusCode::ACCEPTED
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use shepherd_core::db;
    use shepherd_core::db::models::CreateTask;
    use tower::ServiceExt;

    async fn test_state() -> Arc<AppState> {
        // Minimal in-memory state for route tests.
        // The handlers under test only access `db` — other fields use the
        // simplest valid construction. If a field does not implement Default,
        // consult its existing constructor in main.rs and adapt.
        let conn = db::open_memory().unwrap();
        let (event_tx, _) = tokio::sync::broadcast::channel::<shepherd_core::events::ServerEvent>(4);
        Arc::new(AppState {
            db: Arc::new(tokio::sync::Mutex::new(conn)),
            // load_config returns defaults when ~/.shepherd/config.toml is absent
            config: shepherd_core::config::load_config(None)
                .unwrap_or_else(|_| shepherd_core::config::ShepherdConfig::default()),
            adapters: shepherd_core::adapters::AdapterRegistry::new(),
            // YoloEngine::load with non-existent path returns an empty rule set
            yolo: shepherd_core::yolo::YoloEngine::load(
                std::path::Path::new("/tmp/__nonexistent_shepherd_rules.yaml")
            ).unwrap_or_else(|_| shepherd_core::yolo::YoloEngine::empty()),
            pty: shepherd_core::pty::PtyManager::new(
                1,
                shepherd_core::pty::sandbox::SandboxProfile::default(),
            ),
            event_tx,
            llm_provider: None,
            iterm2: None,
        })
        // NOTE: If ShepherdConfig or YoloEngine lack Default/empty() constructors,
        // add a #[cfg(test)] factory method to each type before writing these tests.
    }

    #[tokio::test]
    async fn test_list_claude_sessions_not_found_task() {
        let state = test_state().await;
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/sessions/9999/claude-sessions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_claude_sessions_non_iterm2_task() {
        let state = test_state().await;
        let task_id = {
            let conn = state.db.lock().await;
            shepherd_core::db::queries::create_task(&conn, &CreateTask {
                title: "test".into(),
                prompt: None,
                agent_id: "claude".into(),
                repo_path: Some("/tmp/nonexistent-proj".into()),
                isolation_mode: None,
                iterm2_session_id: None,
            }).unwrap().id
        };
        let app = crate::build_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/sessions/{task_id}/claude-sessions"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["sessions"], serde_json::json!([]));
    }
}
```

- [ ] **Step 3: Run route tests — verify FAIL**

```bash
cargo test -p shepherd-server -- routes::iterm2::tests 2>&1 | tail -8
```

- [ ] **Step 4: Register routes and pub mod**

In `crates/shepherd-server/src/routes/mod.rs`, add:
```rust
pub mod iterm2;
```

In `crates/shepherd-server/src/lib.rs`, add 3 routes:
```rust
.route("/api/sessions/:id/claude-sessions", get(routes::iterm2::list_claude_sessions))
.route("/api/sessions/:id/resume", post(routes::iterm2::resume_session))
.route("/api/sessions/:id/fresh", post(routes::iterm2::fresh_session))
```

- [ ] **Step 5: Run tests — verify PASS**

```bash
cargo test -p shepherd-server -- routes::iterm2::tests 2>&1 | tail -8
```

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-server/src/routes/iterm2.rs \
        crates/shepherd-server/src/routes/mod.rs \
        crates/shepherd-server/src/lib.rs \
        crates/shepherd-server/src/state.rs
git commit -m "feat(server): iTerm2 routes — list_claude_sessions, resume, fresh"
```

---

### Task 10: Initialize Iterm2Manager in main.rs

**Files:**
- Modify: `crates/shepherd-server/src/main.rs`

- [ ] **Step 1: Add iterm2 initialization**

In `crates/shepherd-server/src/main.rs`, before constructing `AppState`:

```rust
use shepherd_core::iterm2::{Iterm2Manager, auth::default_auth_path};
use std::sync::Arc;

// ...existing setup...

let iterm2_auth_path = default_auth_path();
let iterm2 = Arc::new(Iterm2Manager::new(iterm2_auth_path));
```

In the `AppState` construction, add:
```rust
iterm2: Some(iterm2.clone()),
```

Spawn the background loop **between** `let app = shepherd_server::build_router(state.clone());` and `axum::serve(listener, app)`. In `main.rs`, insert:

```rust
// Spawn iTerm2 adoption loop (non-blocking — runs independently)
if let Some(ref mgr) = state.iterm2 {
    mgr.clone().spawn(state.db.clone(), state.event_tx.clone());
}

// existing:
axum::serve(listener, app)
    .with_graceful_shutdown(...)
    .await?;
```

- [ ] **Step 2: Verify the server compiles and starts**

```bash
cargo build -p shepherd-server 2>&1 | tail -5
```

Expected: clean build.

- [ ] **Step 3: Verify all existing tests still pass**

```bash
cargo test -p shepherd-core -p shepherd-server 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/shepherd-server/src/main.rs
git commit -m "feat(server): initialize and spawn Iterm2Manager background loop"
```

---

## Chunk 4: Frontend

### Task 11: Extend Task type with iterm2_session_id

**Files:**
- Modify: `src/store/tasks.ts`

- [ ] **Step 1: Add the field to the frontend Task type**

In `src/store/tasks.ts`, find the `Task` interface and add:

```typescript
iterm2_session_id?: string | null;
```

- [ ] **Step 2: Update the event → Task mapping function**

In `src/store/tasks.ts`, find the function that maps a server `TaskEvent` to a `Task` — it will look something like `taskEventToTask(event: TaskEvent): Task` or an inline object spread in `upsertTask`. Add the field explicitly:

```typescript
// Find the mapping and add:
iterm2_session_id: event.iterm2_session_id ?? null,
```

**This step is required.** The field will always be `undefined` in the store even when the server sends it unless the mapping function explicitly propagates it. TypeScript spread (`{ ...existing, ...event }`) only works if both objects are typed consistently, and `iterm2_session_id` may be omitted from older task events — always default to `null`.

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npm run build 2>&1 | tail -10
```

Expected: no type errors.

- [ ] **Step 4: Commit**

```bash
git add src/store/tasks.ts
git commit -m "feat(frontend): add iterm2_session_id to Task type and event mapping"
```

---

### Task 12: iTerm2 UI components

**Files:**
- Create: `src/features/iterm2/Iterm2Badge.tsx`
- Create: `src/features/iterm2/SessionPicker.tsx`
- Create: `src/features/iterm2/SetupPrompt.tsx`
- Create: `src/features/iterm2/__tests__/iterm2.test.tsx`

- [ ] **Step 1: Write component tests first**

```typescript
// src/features/iterm2/__tests__/iterm2.test.tsx
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { Iterm2Badge } from '../Iterm2Badge';
import { SessionPicker } from '../SessionPicker';
import { SetupPrompt } from '../SetupPrompt';

describe('Iterm2Badge', () => {
  it('renders iTerm2 pill', () => {
    render(<Iterm2Badge />);
    expect(screen.getByText(/iterm2/i)).toBeTruthy();
  });
});

describe('SessionPicker', () => {
  it('renders Resume and Start Fresh buttons', () => {
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc', 'session-def']}
        onResume={vi.fn()}
        onFresh={vi.fn()}
      />
    );
    expect(screen.getByRole('button', { name: /resume/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /start fresh/i })).toBeTruthy();
  });

  it('shows session options in dropdown', () => {
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc', 'session-def']}
        onResume={vi.fn()}
        onFresh={vi.fn()}
      />
    );
    expect(screen.getByText('session-abc')).toBeTruthy();
    expect(screen.getByText('session-def')).toBeTruthy();
  });

  it('renders "No sessions" when list is empty', () => {
    render(
      <SessionPicker
        taskId={1}
        sessions={[]}
        onResume={vi.fn()}
        onFresh={vi.fn()}
      />
    );
    expect(screen.getByText(/no sessions/i)).toBeTruthy();
  });

  it('disables Resume button when no sessions available', () => {
    render(
      <SessionPicker
        taskId={1}
        sessions={[]}
        onResume={vi.fn()}
        onFresh={vi.fn()}
      />
    );
    const resumeBtn = screen.getByRole('button', { name: /resume/i });
    expect(resumeBtn).toBeDisabled();
  });

  it('calls onResume with selected session id', () => {
    const onResume = vi.fn();
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc']}
        onResume={onResume}
        onFresh={vi.fn()}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /resume/i }));
    expect(onResume).toHaveBeenCalledWith('session-abc');
  });

  it('calls onFresh when Start Fresh clicked', () => {
    const onFresh = vi.fn();
    render(
      <SessionPicker
        taskId={1}
        sessions={['session-abc']}
        onResume={vi.fn()}
        onFresh={onFresh}
      />
    );
    fireEvent.click(screen.getByRole('button', { name: /start fresh/i }));
    expect(onFresh).toHaveBeenCalled();
  });
});

describe('SetupPrompt', () => {
  it('renders setup instructions', () => {
    render(<SetupPrompt onDismiss={vi.fn()} />);
    expect(screen.getByText(/iterm2/i)).toBeTruthy();
    expect(screen.getByText(/shepherd-bridge\.py/i)).toBeTruthy();
  });

  it('calls onDismiss when dismissed', () => {
    const onDismiss = vi.fn();
    render(<SetupPrompt onDismiss={onDismiss} />);
    const btn = screen.getByRole('button', { name: /dismiss/i });
    fireEvent.click(btn);
    expect(onDismiss).toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Run tests — verify FAIL**

```bash
npm run test -- src/features/iterm2 2>&1 | tail -10
```

- [ ] **Step 3: Implement the components**

```typescript
// src/features/iterm2/Iterm2Badge.tsx
export function Iterm2Badge() {
  return (
    <span className="inline-flex items-center rounded-full bg-purple-100 px-2 py-0.5 text-xs font-medium text-purple-800 dark:bg-purple-900 dark:text-purple-200">
      iTerm2
    </span>
  );
}
```

```typescript
// src/features/iterm2/SessionPicker.tsx
import { useState } from 'react';

interface Props {
  taskId: number;
  sessions: string[];
  onResume: (sessionId: string) => void;
  onFresh: () => void;
}

export function SessionPicker({ sessions, onResume, onFresh }: Props) {
  const [selected, setSelected] = useState(sessions[0] ?? '');
  const hasSession = sessions.length > 0;

  return (
    <div className="flex flex-col gap-2">
      {hasSession ? (
        <select
          className="rounded border px-2 py-1 text-sm"
          value={selected}
          onChange={e => setSelected(e.target.value)}
        >
          {sessions.map(s => (
            <option key={s} value={s}>{s}</option>
          ))}
        </select>
      ) : (
        <p className="text-sm text-muted-foreground">No sessions available</p>
      )}
      <div className="flex gap-2">
        <button
          className="rounded bg-blue-600 px-3 py-1 text-sm text-white disabled:opacity-50"
          disabled={!hasSession}
          onClick={() => onResume(selected)}
        >
          Resume
        </button>
        <button
          className="rounded border px-3 py-1 text-sm"
          onClick={onFresh}
        >
          Start Fresh
        </button>
      </div>
    </div>
  );
}
```

```typescript
// src/features/iterm2/SetupPrompt.tsx
interface Props {
  onDismiss: () => void;
}

export function SetupPrompt({ onDismiss }: Props) {
  return (
    <div className="rounded-lg border border-yellow-300 bg-yellow-50 p-4 dark:border-yellow-700 dark:bg-yellow-950">
      <h3 className="font-semibold text-yellow-800 dark:text-yellow-200">
        Enable iTerm2 Integration
      </h3>
      <p className="mt-1 text-sm text-yellow-700 dark:text-yellow-300">
        To adopt existing iTerm2 sessions, install the Shepherd bridge script:
      </p>
      <ol className="mt-2 list-decimal pl-5 text-sm text-yellow-700 dark:text-yellow-300">
        <li>Enable the Python API in iTerm2 → Preferences → General → Magic</li>
        <li>
          Copy <code className="rounded bg-yellow-100 px-1 dark:bg-yellow-900">shepherd-bridge.py</code> to{' '}
          <code className="rounded bg-yellow-100 px-1 dark:bg-yellow-900">
            ~/Library/Application Support/iTerm2/Scripts/AutoLaunch/
          </code>
        </li>
        <li>Restart iTerm2</li>
      </ol>
      <button
        className="mt-3 text-xs text-yellow-600 underline dark:text-yellow-400"
        onClick={onDismiss}
        aria-label="Dismiss"
      >
        Dismiss
      </button>
    </div>
  );
}
```

- [ ] **Step 4: Run tests — verify PASS**

```bash
npm run test -- src/features/iterm2 2>&1 | tail -10
```

Expected: 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/features/iterm2/
git commit -m "feat(frontend): Iterm2Badge, SessionPicker, SetupPrompt components"
```

---

### Task 13: Mount iTerm2 components in task card and detail

**Files:**
- Modify: Task card component (find with `grep -r "iterm2\|task_id\|TaskCard" src/features --include="*.tsx" -l`)
- Modify: Task detail component (same grep)

- [ ] **Step 1: Find the task card component**

```bash
grep -rn "agent_id\|repo_path\|TaskCard\|task\.status" src/features --include="*.tsx" -l
```

Open the file(s) found. Identify where the task title is rendered.

- [ ] **Step 2: Add Iterm2Badge to task card**

In the task card JSX, after the title, add:

```tsx
import { Iterm2Badge } from '../iterm2/Iterm2Badge';

// In render, alongside task title:
{task.iterm2_session_id && <Iterm2Badge />}
```

- [ ] **Step 3: Find the task detail/panel component and add SessionPicker**

In the task detail component, add the session picker (fetching sessions via API):

```tsx
import { useState, useEffect } from 'react';
import { SessionPicker } from '../iterm2/SessionPicker';

// Inside the component, when task.iterm2_session_id is set:
const [claudeSessions, setClaudeSessions] = useState<string[]>([]);

useEffect(() => {
  if (!task.iterm2_session_id) return;
  fetch(`/api/sessions/${task.id}/claude-sessions`)
    .then(r => r.json())
    .then(data => setClaudeSessions(data.sessions ?? []));
}, [task.id, task.iterm2_session_id]);

// In JSX:
{task.iterm2_session_id && (
  <SessionPicker
    taskId={task.id}
    sessions={claudeSessions}
    onResume={sessionId =>
      fetch(`/api/sessions/${task.id}/resume`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ claude_session_id: sessionId }),
      })
    }
    onFresh={() =>
      fetch(`/api/sessions/${task.id}/fresh`, { method: 'POST' })
    }
  />
)}
```

- [ ] **Step 4: Mount SetupPrompt with client-side dismissal**

Show `SetupPrompt` when the task has an `iterm2_session_id` but no Claude sessions are available yet (which happens both when the auth file is missing and when the CWD has no prior sessions). This is purely client-side — no extra endpoint needed. Add a local dismissed state and render above the `SessionPicker`:

```tsx
const [setupDismissed, setSetupDismissed] = useState(false);

{task.iterm2_session_id && !setupDismissed && claudeSessions.length === 0 && (
  <SetupPrompt onDismiss={() => setSetupDismissed(true)} />
)}
```

Once the user installs the bridge and restarts iTerm2, the next scan will populate `iterm2-auth.json`, sessions will be adopted, and the `claude-sessions` endpoint will return results — at which point the prompt disappears naturally.

- [ ] **Step 5: Run full test suite**

```bash
npm run test 2>&1 | tail -10
cargo test -p shepherd-core -p shepherd-server 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 6: Build check**

```bash
npm run build 2>&1 | tail -5
cargo build 2>&1 | tail -5
```

Expected: clean builds.

- [ ] **Step 7: Commit**

```bash
git add src/
git commit -m "feat(frontend): mount Iterm2Badge, SessionPicker, SetupPrompt in task UI"
```

---

### Task 14: Ship the AutoLaunch bridge script

**Files:**
- Create: `scripts/shepherd-bridge.py`

- [ ] **Step 1: Write the bridge script**

```python
#!/usr/bin/env python3
"""
shepherd-bridge.py — iTerm2 AutoLaunch bridge

Must be installed as an iTerm2 Python API AutoLaunch script so the iTerm2
runtime injects ITERM2_COOKIE and ITERM2_KEY into the process environment.
A plain Python script NOT invoked via iterm2.run_until_complete() will NOT
receive these env vars.

Install at:
  ~/Library/Application Support/iTerm2/Scripts/AutoLaunch/shepherd-bridge.py

iTerm2 must have the Python API enabled:
  Preferences → General → Magic → Enable Python API
"""
import iterm2   # provided by iTerm2's embedded Python environment
import json
import os
import pathlib
import stat


async def main(connection):
    cookie = os.environ.get("ITERM2_COOKIE", "")
    key = os.environ.get("ITERM2_KEY", "")

    if not cookie or not key:
        print("shepherd-bridge: ITERM2_COOKIE/KEY not available")
        return

    auth_dir = pathlib.Path.home() / ".shepherd"
    auth_dir.mkdir(parents=True, exist_ok=True)
    auth_path = auth_dir / "iterm2-auth.json"
    auth_path.write_text(json.dumps({"cookie": cookie, "key": key}))
    auth_path.chmod(stat.S_IRUSR | stat.S_IWUSR)  # 0600
    print(f"shepherd-bridge: credentials written to {auth_path}")


iterm2.run_until_complete(main)
```

- [ ] **Step 2: Commit**

```bash
git add scripts/shepherd-bridge.py
git commit -m "feat: add iTerm2 AutoLaunch bridge script for credential forwarding"
```

---

## Verification

```bash
# All Rust tests
cargo test -p shepherd-core -p shepherd-server 2>&1 | tail -20

# Rust coverage (target: iterm2 module at >85%)
cargo tarpaulin -p shepherd-core --out Stdout 2>&1 | grep -E "iterm2|Coverage"

# Frontend tests
npm run test 2>&1 | tail -10

# Full build
cargo build && npm run build
```

---

## Notes for Implementers

1. **Proto enum paths:** After vendoring the proto and running `cargo build`, inspect `target/debug/build/shepherd-core-*/out/iterm2.rs` to find the exact generated enum variant names for `NotificationType`. The proto uses `enum NotificationType` inside a message — prost may generate it as a module or as a top-level enum. Adjust the scanner's `notification_type` field accordingly.

2. **SplitTreeNode shape:** The exact proto shape of `SplitTreeNode` (whether children are `repeated SplitTreeNode` or a `oneof`) depends on the vendored proto version. Inspect `iterm2.rs` and adjust `walk_node` in `scanner.rs`.

3. **GetBufferRequest field name:** The spec notes the field name should be confirmed against the vendored proto. Look for a field meaning "last N lines" — likely `trailing_lines`, `num_lines`, or `first_line`/`last_line`. Use the correct field.

4. **Row mapping in queries.rs:** The `row_to_task` helper must include `iterm2_session_id` at the correct column index. After adding the migration, `iterm2_session_id` is the 11th column (index 10) in `SELECT ... FROM tasks`.

5. **TaskEvent in events.rs:** The existing `TaskEvent` struct is used across the codebase. Adding `iterm2_session_id: Option<String>` is a backward-compatible field change — all existing construction sites should use `iterm2_session_id: None`.
