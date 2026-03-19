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

    // Idempotent: silently ignored if column already exists
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN iterm2_session_id TEXT",
        [],
    ).ok();

    // Context orchestrator tables
    crate::context::feedback::migrate(conn)?;
    crate::context::index::migrate(conn)?;

    // Observability tables
    crate::observability::store::migrate(conn)?;

    // Session replay tables
    crate::replay::migrate(conn)?;

    Ok(())
}

pub fn update_task_status(conn: &Connection, task_id: i64, status: models::TaskStatus) -> Result<()> {
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
            status: models::TaskStatus::parse_status(&row.get::<_, String>(7)?).unwrap_or(models::TaskStatus::Queued),
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            iterm2_session_id: row.get(10)?,
        })
    })?.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::TaskStatus;

    #[test]
    fn test_open_memory_creates_tables() {
        let conn = open_memory().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('tasks', 'sessions', 'permissions', 'diffs', 'profiles', 'gate_results', 'context_packages', 'context_feedback', 'task_metrics', 'file_index', 'session_events')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 11);
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

    #[test]
    fn test_open_file_based_db() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = open(&db_path).unwrap();
        // Verify tables are created
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_migrate_idempotent() {
        let conn = open_memory().unwrap();
        // Running migrate again should not fail
        migrate(&conn).unwrap();
        // Verify tables still exist
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let conn = open_memory().unwrap();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

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

    #[test]
    fn test_update_task_status() {
        let conn = open_memory().unwrap();
        conn.execute(
            "INSERT INTO tasks (title, prompt, agent_id, repo_path, branch, isolation_mode, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["Test", "Do thing", "claude-code", "/tmp", "main", "worktree", "queued"],
        ).unwrap();
        update_task_status(&conn, 1, TaskStatus::Dispatching).unwrap();
        let status: String = conn
            .query_row(
                "SELECT status FROM tasks WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "dispatching");
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
}
