use anyhow::Result;
use rusqlite::{params, Connection, Row};

use super::models::{CreateTask, Task, TaskStatus};

/// Map a database row to a `Task` struct.
///
/// Expects columns in order: id, title, prompt, agent_id, repo_path,
/// branch, isolation_mode, status, created_at, updated_at.
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
    })
}

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
        row_to_task,
    )?;
    Ok(task)
}

pub fn list_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, prompt, agent_id, repo_path, branch, isolation_mode, status, created_at, updated_at FROM tasks ORDER BY id"
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
            },
        )
        .unwrap();

        assert_eq!(list_tasks(&conn).unwrap().len(), 1);

        delete_task(&conn, task.id).unwrap();

        let tasks = list_tasks(&conn).unwrap();
        assert!(tasks.is_empty());
    }
}
