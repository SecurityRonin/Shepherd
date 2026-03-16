use anyhow::Result;
use rusqlite::{params, Connection, Row};

use super::models::{CreateTask, Task, TaskStatus};

/// Map a database row to a `Task` struct.
///
/// Expects columns in order: id, title, prompt, agent_id, repo_path,
/// branch, isolation_mode, status, created_at, updated_at, iterm2_session_id.
fn row_to_task(row: &Row) -> rusqlite::Result<Task> {
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

pub fn create_task(conn: &Connection, input: &CreateTask) -> Result<Task> {
    conn.execute(
        "INSERT INTO tasks (title, prompt, agent_id, repo_path, isolation_mode, iterm2_session_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.title,
            input.prompt.as_deref().unwrap_or(""),
            input.agent_id,
            input.repo_path.as_deref().unwrap_or(""),
            input.isolation_mode.as_deref().unwrap_or("worktree"),
            input.iterm2_session_id,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get_task(conn, id)
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Task> {
    let task = conn.query_row(
        "SELECT id, title, prompt, agent_id, repo_path, branch, isolation_mode, status, created_at, updated_at, iterm2_session_id FROM tasks WHERE id = ?1",
        params![id],
        row_to_task,
    )?;
    Ok(task)
}

pub fn list_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, prompt, agent_id, repo_path, branch, isolation_mode, status, created_at, updated_at, iterm2_session_id FROM tasks ORDER BY id"
    )?;
    let tasks = stmt.query_map([], row_to_task)?
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

pub fn find_task_by_iterm2_id(
    conn: &Connection,
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
                iterm2_session_id: None,
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
            iterm2_session_id: None,
        }).unwrap();
        create_task(&conn, &CreateTask {
            title: "Task 2".into(),
            prompt: None,
            agent_id: "codex".into(),
            repo_path: None,
            isolation_mode: None,
            iterm2_session_id: None,
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
            iterm2_session_id: None,
        }).unwrap();

        update_task_status(&conn, task.id, &TaskStatus::Running).unwrap();
        let updated = get_task(&conn, task.id).unwrap();
        assert_eq!(updated.status, TaskStatus::Running);
    }

    #[test]
    fn test_count_by_status() {
        let conn = open_memory().unwrap();
        create_task(&conn, &CreateTask {
            title: "T1".into(), prompt: None, agent_id: "a".into(), repo_path: None, isolation_mode: None, iterm2_session_id: None,
        }).unwrap();
        create_task(&conn, &CreateTask {
            title: "T2".into(), prompt: None, agent_id: "a".into(), repo_path: None, isolation_mode: None, iterm2_session_id: None,
        }).unwrap();

        let counts = count_by_status(&conn).unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0], ("queued".to_string(), 2));
    }

    #[test]
    fn test_delete_task() {
        let conn = open_memory().unwrap();
        let task = create_task(
            &conn,
            &CreateTask {
                title: "To be deleted".into(),
                prompt: Some("Delete me".into()),
                agent_id: "claude-code".into(),
                repo_path: None,
                isolation_mode: None,
                iterm2_session_id: None,
            },
        )
        .unwrap();

        assert_eq!(list_tasks(&conn).unwrap().len(), 1);

        delete_task(&conn, task.id).unwrap();

        let tasks = list_tasks(&conn).unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_find_task_by_iterm2_id_found() {
        let conn = open_memory().unwrap();
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
        let conn = open_memory().unwrap();
        let result = find_task_by_iterm2_id(&conn, "no-such-id").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update_task_status_changes_status() {
        use crate::db::models::TaskStatus;
        let conn = open_memory().unwrap();
        let task = create_task(&conn, &CreateTask {
            title: "status test".into(),
            prompt: None,
            agent_id: "claude".into(),
            repo_path: None,
            isolation_mode: None,
            iterm2_session_id: None,
        }).unwrap();
        update_task_status(&conn, task.id, &TaskStatus::Done).unwrap();
        let updated = get_task(&conn, task.id).unwrap();
        assert_eq!(updated.status, TaskStatus::Done);
    }
}
