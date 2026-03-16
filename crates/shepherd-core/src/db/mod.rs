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

#[cfg(test)]
mod tests {
    use super::*;

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
}
