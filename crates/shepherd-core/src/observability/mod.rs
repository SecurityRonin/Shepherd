//! Observability — cost tracking, budget enforcement, and spending analytics.
//!
//! Provides:
//! - Model pricing tables with per-token cost calculation
//! - Per-task metrics accumulation (tokens, cost, duration)
//! - Budget limits (per-task, per-agent-daily, global-daily) with alerts
//! - SQLite persistence for metrics and spending summaries

pub mod budget;
pub mod metrics;
pub mod pricing;
pub mod store;

pub use budget::{BudgetAlert, BudgetConfig, BudgetScope, BudgetStatus};
pub use metrics::{
    AgentSpending, CostEstimate, MetricsAccumulator, ModelSpending, SpendingSummary, TaskMetrics,
};
pub use pricing::{estimate_cost, find_pricing, ModelPricing};

/// Check all budget limits for a running task and return any alerts.
///
/// This is the main entry point for budget enforcement. Call it after
/// each LLM call to check whether any spending limits have been hit.
pub fn check_budgets(
    config: &BudgetConfig,
    conn: &rusqlite::Connection,
    task_cost: f64,
    task_id: &str,
    agent_id: &str,
) -> Vec<BudgetAlert> {
    let agent_daily = store::get_agent_daily_cost(conn, agent_id).unwrap_or(0.0);
    let global_daily = store::get_global_daily_cost(conn).unwrap_or(0.0);

    budget::check_task_budgets(
        config,
        task_cost,
        agent_daily,
        global_daily,
        task_id,
        agent_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn re_exports_are_accessible() {
        // Verify the public API is usable through the mod re-exports
        let _config = BudgetConfig::default();
        let _est = CostEstimate::from_usage("claude-sonnet-4", 1000, 500);
        let _pricing = find_pricing("claude-sonnet-4");
        let cost = estimate_cost("claude-sonnet-4", 1000, 500);
        assert!(cost > 0.0);
    }

    #[test]
    fn check_budgets_integration() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        store::migrate(&conn).unwrap();

        // Insert some usage
        let mut acc = MetricsAccumulator::new(1, "claude-code");
        acc.record("claude-sonnet-4", 50_000, 10_000);
        let metrics = acc.finalize("done");
        store::upsert_metrics(&conn, &metrics).unwrap();

        // Check with generous limits — no alerts
        let config = BudgetConfig {
            max_cost_per_task: Some(100.0),
            max_cost_per_agent_daily: Some(500.0),
            max_cost_daily: Some(1000.0),
            warning_threshold: 0.8,
        };
        let alerts = check_budgets(&config, &conn, metrics.total_cost_usd, "1", "claude-code");
        assert!(alerts.is_empty());
    }

    #[test]
    fn check_budgets_triggers_alert() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        store::migrate(&conn).unwrap();

        // Insert some usage
        let mut acc = MetricsAccumulator::new(1, "claude-code");
        acc.record("claude-sonnet-4", 50_000, 10_000);
        let metrics = acc.finalize("done");
        store::upsert_metrics(&conn, &metrics).unwrap();

        // Set a very tight task budget
        let config = BudgetConfig {
            max_cost_per_task: Some(0.01),
            max_cost_per_agent_daily: None,
            max_cost_daily: None,
            warning_threshold: 0.8,
        };
        let alerts = check_budgets(&config, &conn, metrics.total_cost_usd, "1", "claude-code");
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].status, BudgetStatus::Exceeded);
    }

    #[test]
    fn spending_summary_via_store() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        store::migrate(&conn).unwrap();

        let mut acc1 = MetricsAccumulator::new(1, "claude-code");
        acc1.record("claude-sonnet-4", 10_000, 5_000);
        store::upsert_metrics(&conn, &acc1.finalize("done")).unwrap();

        let mut acc2 = MetricsAccumulator::new(2, "codex");
        acc2.record("gpt-4o", 8_000, 3_000);
        store::upsert_metrics(&conn, &acc2.finalize("done")).unwrap();

        let summary = store::get_spending_summary(&conn).unwrap();
        assert_eq!(summary.total_tasks, 2);
        assert!(summary.total_cost_usd > 0.0);
        assert_eq!(summary.by_agent.len(), 2);
        assert_eq!(summary.by_model.len(), 2);
    }
}
