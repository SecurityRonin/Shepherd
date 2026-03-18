use anyhow::Result;
use rusqlite::{params, Connection};
use super::metrics::{TaskMetrics, SpendingSummary, AgentSpending, ModelSpending};

/// Create observability tables if they don't exist.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS task_metrics (
            task_id INTEGER PRIMARY KEY,
            agent_id TEXT NOT NULL,
            model_id TEXT NOT NULL DEFAULT '',
            total_input_tokens INTEGER NOT NULL DEFAULT 0,
            total_output_tokens INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            total_cost_usd REAL NOT NULL DEFAULT 0.0,
            llm_calls INTEGER NOT NULL DEFAULT 0,
            duration_secs REAL,
            status TEXT NOT NULL DEFAULT 'running',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_task_metrics_agent
            ON task_metrics(agent_id);
        CREATE INDEX IF NOT EXISTS idx_task_metrics_created
            ON task_metrics(created_at);
        ",
    )?;
    Ok(())
}

/// Upsert task metrics (insert or update).
pub fn upsert_metrics(conn: &Connection, metrics: &TaskMetrics) -> Result<()> {
    conn.execute(
        "INSERT INTO task_metrics (task_id, agent_id, model_id, total_input_tokens, total_output_tokens, total_tokens, total_cost_usd, llm_calls, duration_secs, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(task_id) DO UPDATE SET
            model_id = excluded.model_id,
            total_input_tokens = excluded.total_input_tokens,
            total_output_tokens = excluded.total_output_tokens,
            total_tokens = excluded.total_tokens,
            total_cost_usd = excluded.total_cost_usd,
            llm_calls = excluded.llm_calls,
            duration_secs = excluded.duration_secs,
            status = excluded.status,
            updated_at = excluded.updated_at",
        params![
            metrics.task_id,
            metrics.agent_id,
            metrics.model_id,
            metrics.total_input_tokens as i64,
            metrics.total_output_tokens as i64,
            metrics.total_tokens as i64,
            metrics.total_cost_usd,
            metrics.llm_calls,
            metrics.duration_secs,
            metrics.status,
            metrics.created_at,
            metrics.updated_at,
        ],
    )?;
    Ok(())
}

/// Load metrics for a specific task.
pub fn get_task_metrics(conn: &Connection, task_id: i64) -> Result<Option<TaskMetrics>> {
    let mut stmt = conn.prepare(
        "SELECT task_id, agent_id, model_id, total_input_tokens, total_output_tokens, total_tokens, total_cost_usd, llm_calls, duration_secs, status, created_at, updated_at
         FROM task_metrics WHERE task_id = ?1",
    )?;

    let result = stmt.query_row(params![task_id], row_to_metrics);
    match result {
        Ok(m) => Ok(Some(m)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        // tarpaulin-start-ignore
        Err(e) => Err(e.into()),
        // tarpaulin-stop-ignore
    }
}

/// Get total cost for an agent today.
pub fn get_agent_daily_cost(conn: &Connection, agent_id: &str) -> Result<f64> {
    let cost: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total_cost_usd), 0.0) FROM task_metrics
             WHERE agent_id = ?1 AND DATE(created_at) = DATE('now')",
            params![agent_id],
            |row| row.get(0),
        )?;
    Ok(cost)
}

/// Get total cost across all agents today.
pub fn get_global_daily_cost(conn: &Connection) -> Result<f64> {
    let cost: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(total_cost_usd), 0.0) FROM task_metrics
             WHERE DATE(created_at) = DATE('now')",
            [],
            |row| row.get(0),
        )?;
    Ok(cost)
}

/// Get a spending summary across all stored metrics.
pub fn get_spending_summary(conn: &Connection) -> Result<SpendingSummary> {
    // Totals
    let (total_cost, total_tokens, total_tasks, total_calls): (f64, i64, u32, u32) = conn
        .query_row(
            "SELECT COALESCE(SUM(total_cost_usd), 0.0), COALESCE(SUM(total_tokens), 0), COUNT(*), COALESCE(SUM(llm_calls), 0) FROM task_metrics",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

    // By agent
    let mut stmt = conn.prepare(
        "SELECT agent_id, SUM(total_cost_usd), SUM(total_tokens), COUNT(*)
         FROM task_metrics GROUP BY agent_id ORDER BY SUM(total_cost_usd) DESC",
    )?;
    let by_agent: Vec<AgentSpending> = stmt
        .query_map([], |row| {
            Ok(AgentSpending {
                agent_id: row.get(0)?,
                total_cost_usd: row.get(1)?,
                total_tokens: row.get::<_, i64>(2)? as u64,
                task_count: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // By model
    let mut stmt = conn.prepare(
        "SELECT model_id, SUM(total_cost_usd), SUM(total_tokens), SUM(llm_calls)
         FROM task_metrics WHERE model_id != '' GROUP BY model_id ORDER BY SUM(total_cost_usd) DESC",
    )?;
    let by_model: Vec<ModelSpending> = stmt
        .query_map([], |row| {
            Ok(ModelSpending {
                model_id: row.get(0)?,
                total_cost_usd: row.get(1)?,
                total_tokens: row.get::<_, i64>(2)? as u64,
                call_count: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(SpendingSummary {
        total_cost_usd: total_cost,
        total_tokens: total_tokens as u64,
        total_tasks,
        total_llm_calls: total_calls,
        by_agent,
        by_model,
    })
}

fn row_to_metrics(row: &rusqlite::Row) -> rusqlite::Result<TaskMetrics> {
    Ok(TaskMetrics {
        task_id: row.get(0)?,
        agent_id: row.get(1)?,
        model_id: row.get(2)?,
        total_input_tokens: row.get::<_, i64>(3)? as u64,
        total_output_tokens: row.get::<_, i64>(4)? as u64,
        total_tokens: row.get::<_, i64>(5)? as u64,
        total_cost_usd: row.get(6)?,
        llm_calls: row.get(7)?,
        duration_secs: row.get(8)?,
        status: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::metrics::MetricsAccumulator;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn sample_metrics(task_id: i64, agent: &str, model: &str, cost: f64) -> TaskMetrics {
        TaskMetrics {
            task_id,
            agent_id: agent.to_string(),
            model_id: model.to_string(),
            total_input_tokens: 10_000,
            total_output_tokens: 5_000,
            total_tokens: 15_000,
            total_cost_usd: cost,
            llm_calls: 3,
            duration_secs: Some(45.0),
            status: "done".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn migrate_creates_table() {
        let conn = setup_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_metrics'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migrate_idempotent() {
        let conn = setup_db();
        migrate(&conn).unwrap();
    }

    #[test]
    fn upsert_and_get_metrics() {
        let conn = setup_db();
        let metrics = sample_metrics(1, "claude-code", "claude-sonnet-4", 0.5);
        upsert_metrics(&conn, &metrics).unwrap();

        let loaded = get_task_metrics(&conn, 1).unwrap().unwrap();
        assert_eq!(loaded.task_id, 1);
        assert_eq!(loaded.agent_id, "claude-code");
        assert_eq!(loaded.total_tokens, 15_000);
        assert!((loaded.total_cost_usd - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn upsert_updates_existing() {
        let conn = setup_db();
        let m1 = sample_metrics(1, "claude-code", "claude-sonnet-4", 0.5);
        upsert_metrics(&conn, &m1).unwrap();

        let m2 = TaskMetrics {
            total_cost_usd: 1.5,
            llm_calls: 10,
            status: "done".to_string(),
            ..m1
        };
        upsert_metrics(&conn, &m2).unwrap();

        let loaded = get_task_metrics(&conn, 1).unwrap().unwrap();
        assert!((loaded.total_cost_usd - 1.5).abs() < f64::EPSILON);
        assert_eq!(loaded.llm_calls, 10);
    }

    #[test]
    fn get_nonexistent_metrics() {
        let conn = setup_db();
        let result = get_task_metrics(&conn, 999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn agent_daily_cost() {
        let conn = setup_db();
        upsert_metrics(&conn, &sample_metrics(1, "claude-code", "sonnet", 0.5)).unwrap();
        upsert_metrics(&conn, &sample_metrics(2, "claude-code", "sonnet", 0.3)).unwrap();
        upsert_metrics(&conn, &sample_metrics(3, "codex", "gpt-4o", 0.2)).unwrap();

        let cost = get_agent_daily_cost(&conn, "claude-code").unwrap();
        assert!((cost - 0.8).abs() < 1e-10);
    }

    #[test]
    fn global_daily_cost() {
        let conn = setup_db();
        upsert_metrics(&conn, &sample_metrics(1, "claude-code", "sonnet", 0.5)).unwrap();
        upsert_metrics(&conn, &sample_metrics(2, "codex", "gpt-4o", 0.3)).unwrap();

        let cost = get_global_daily_cost(&conn).unwrap();
        assert!((cost - 0.8).abs() < 1e-10);
    }

    #[test]
    fn spending_summary_totals() {
        let conn = setup_db();
        upsert_metrics(&conn, &sample_metrics(1, "claude-code", "claude-sonnet-4", 0.5)).unwrap();
        upsert_metrics(&conn, &sample_metrics(2, "claude-code", "claude-sonnet-4", 0.3)).unwrap();
        upsert_metrics(&conn, &sample_metrics(3, "codex", "gpt-4o", 0.2)).unwrap();

        let summary = get_spending_summary(&conn).unwrap();
        assert!((summary.total_cost_usd - 1.0).abs() < 1e-10);
        assert_eq!(summary.total_tasks, 3);
        assert_eq!(summary.total_tokens, 45_000); // 15k * 3
    }

    #[test]
    fn spending_summary_by_agent() {
        let conn = setup_db();
        upsert_metrics(&conn, &sample_metrics(1, "claude-code", "sonnet", 0.5)).unwrap();
        upsert_metrics(&conn, &sample_metrics(2, "claude-code", "sonnet", 0.3)).unwrap();
        upsert_metrics(&conn, &sample_metrics(3, "codex", "gpt-4o", 0.2)).unwrap();

        let summary = get_spending_summary(&conn).unwrap();
        assert_eq!(summary.by_agent.len(), 2);
        // Sorted by cost desc
        assert_eq!(summary.by_agent[0].agent_id, "claude-code");
        assert_eq!(summary.by_agent[0].task_count, 2);
    }

    #[test]
    fn spending_summary_by_model() {
        let conn = setup_db();
        upsert_metrics(&conn, &sample_metrics(1, "claude-code", "claude-sonnet-4", 0.5)).unwrap();
        upsert_metrics(&conn, &sample_metrics(2, "codex", "gpt-4o", 0.3)).unwrap();

        let summary = get_spending_summary(&conn).unwrap();
        assert_eq!(summary.by_model.len(), 2);
    }

    #[test]
    fn spending_summary_empty_db() {
        let conn = setup_db();
        let summary = get_spending_summary(&conn).unwrap();
        assert!((summary.total_cost_usd).abs() < f64::EPSILON);
        assert_eq!(summary.total_tasks, 0);
        assert!(summary.by_agent.is_empty());
        assert!(summary.by_model.is_empty());
    }

    #[test]
    fn accumulator_to_db_roundtrip() {
        let conn = setup_db();
        let mut acc = MetricsAccumulator::new(42, "claude-code");
        acc.record("claude-sonnet-4", 5_000, 2_000);
        acc.record("claude-sonnet-4", 3_000, 1_000);

        let metrics = acc.finalize("done");
        upsert_metrics(&conn, &metrics).unwrap();

        let loaded = get_task_metrics(&conn, 42).unwrap().unwrap();
        assert_eq!(loaded.agent_id, "claude-code");
        assert_eq!(loaded.total_input_tokens, 8_000);
        assert_eq!(loaded.total_output_tokens, 3_000);
        assert_eq!(loaded.llm_calls, 2);
        assert!(loaded.total_cost_usd > 0.0);
    }

    #[test]
    fn agent_daily_cost_no_entries_returns_zero() {
        let conn = setup_db();
        let cost = get_agent_daily_cost(&conn, "nonexistent-agent").unwrap();
        assert!((cost).abs() < f64::EPSILON);
    }

    #[test]
    fn global_daily_cost_empty_db_returns_zero() {
        let conn = setup_db();
        let cost = get_global_daily_cost(&conn).unwrap();
        assert!((cost).abs() < f64::EPSILON);
    }

    #[test]
    fn spending_summary_empty_model_id_excluded_from_by_model() {
        let conn = setup_db();
        // Insert a metrics row with an empty model_id
        let metrics_no_model = sample_metrics(1, "claude-code", "", 0.5);
        upsert_metrics(&conn, &metrics_no_model).unwrap();

        let summary = get_spending_summary(&conn).unwrap();
        // Total tasks should include it
        assert_eq!(summary.total_tasks, 1);
        // But by_model should exclude the empty model_id row
        assert!(summary.by_model.is_empty());
    }

    #[test]
    fn spending_summary_llm_calls_summed() {
        let conn = setup_db();
        upsert_metrics(&conn, &sample_metrics(1, "claude-code", "claude-sonnet-4", 0.5)).unwrap();
        upsert_metrics(&conn, &sample_metrics(2, "claude-code", "claude-sonnet-4", 0.3)).unwrap();

        let summary = get_spending_summary(&conn).unwrap();
        // Each sample_metrics has llm_calls=3, so total = 6
        assert_eq!(summary.total_llm_calls, 6);
    }

    #[test]
    fn migrate_runs_twice_without_error() {
        let conn = setup_db();
        // Second call should not fail
        migrate(&conn).unwrap();
    }
}
