use serde::{Deserialize, Serialize};

/// Budget configuration for cost control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum cost per task in USD. None = unlimited.
    pub max_cost_per_task: Option<f64>,
    /// Maximum cost per agent per day in USD. None = unlimited.
    pub max_cost_per_agent_daily: Option<f64>,
    /// Global daily budget in USD. None = unlimited.
    pub max_cost_daily: Option<f64>,
    /// Warning threshold as a fraction (0.0-1.0) of the limit.
    /// Alert when spending exceeds this fraction.
    pub warning_threshold: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_cost_per_task: None,
            max_cost_per_agent_daily: None,
            max_cost_daily: None,
            warning_threshold: 0.8,
        }
    }
}

/// Current budget status for a scope (task, agent, or global).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStatus {
    /// Under budget, no concerns.
    Ok,
    /// Approaching budget limit (past warning threshold).
    Warning,
    /// At or over budget limit.
    Exceeded,
    /// No budget configured (unlimited).
    Unlimited,
}

/// A budget alert to be sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAlert {
    pub scope: BudgetScope,
    pub scope_id: String,
    pub status: BudgetStatus,
    pub current_cost: f64,
    pub limit: f64,
    pub percentage: f64,
    pub message: String,
}

/// What scope a budget applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetScope {
    Task,
    AgentDaily,
    GlobalDaily,
}

impl std::fmt::Display for BudgetScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetScope::Task => write!(f, "task"),
            BudgetScope::AgentDaily => write!(f, "agent_daily"),
            BudgetScope::GlobalDaily => write!(f, "global_daily"),
        }
    }
}

/// Check if spending exceeds a budget limit.
pub fn check_budget(
    current_cost: f64,
    limit: Option<f64>,
    threshold: f64,
    scope: BudgetScope,
    scope_id: &str,
) -> Option<BudgetAlert> {
    let limit = match limit {
        Some(l) if l > 0.0 => l,
        _ => return None, // No limit configured
    };

    let percentage = current_cost / limit;

    if percentage >= 1.0 {
        Some(BudgetAlert {
            scope,
            scope_id: scope_id.to_string(),
            status: BudgetStatus::Exceeded,
            current_cost,
            limit,
            percentage,
            message: format!(
                "Budget exceeded: ${:.4} / ${:.2} ({:.0}%)",
                current_cost,
                limit,
                percentage * 100.0
            ),
        })
    } else if percentage >= threshold {
        Some(BudgetAlert {
            scope,
            scope_id: scope_id.to_string(),
            status: BudgetStatus::Warning,
            current_cost,
            limit,
            percentage,
            message: format!(
                "Approaching budget: ${:.4} / ${:.2} ({:.0}%)",
                current_cost,
                limit,
                percentage * 100.0
            ),
        })
    } else {
        None // Under threshold, no alert needed
    }
}

/// Check all budget limits for a task.
pub fn check_task_budgets(
    config: &BudgetConfig,
    task_cost: f64,
    agent_daily_cost: f64,
    global_daily_cost: f64,
    task_id: &str,
    agent_id: &str,
) -> Vec<BudgetAlert> {
    let mut alerts = Vec::new();

    if let Some(alert) = check_budget(
        task_cost,
        config.max_cost_per_task,
        config.warning_threshold,
        BudgetScope::Task,
        task_id,
    ) {
        alerts.push(alert);
    }

    if let Some(alert) = check_budget(
        agent_daily_cost,
        config.max_cost_per_agent_daily,
        config.warning_threshold,
        BudgetScope::AgentDaily,
        agent_id,
    ) {
        alerts.push(alert);
    }

    if let Some(alert) = check_budget(
        global_daily_cost,
        config.max_cost_daily,
        config.warning_threshold,
        BudgetScope::GlobalDaily,
        "global",
    ) {
        alerts.push(alert);
    }

    alerts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_limits() {
        let config = BudgetConfig::default();
        assert!(config.max_cost_per_task.is_none());
        assert!(config.max_cost_per_agent_daily.is_none());
        assert!(config.max_cost_daily.is_none());
        assert!((config.warning_threshold - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn check_budget_no_limit() {
        let alert = check_budget(100.0, None, 0.8, BudgetScope::Task, "1");
        assert!(alert.is_none());
    }

    #[test]
    fn check_budget_under_threshold() {
        let alert = check_budget(0.5, Some(10.0), 0.8, BudgetScope::Task, "1");
        assert!(alert.is_none()); // 5% < 80%
    }

    #[test]
    fn check_budget_warning() {
        let alert = check_budget(8.5, Some(10.0), 0.8, BudgetScope::Task, "1").unwrap();
        assert_eq!(alert.status, BudgetStatus::Warning);
        assert!((alert.percentage - 0.85).abs() < f64::EPSILON);
        assert!(alert.message.contains("Approaching"));
    }

    #[test]
    fn check_budget_exceeded() {
        let alert = check_budget(12.0, Some(10.0), 0.8, BudgetScope::Task, "1").unwrap();
        assert_eq!(alert.status, BudgetStatus::Exceeded);
        assert!(alert.percentage >= 1.0);
        assert!(alert.message.contains("exceeded"));
    }

    #[test]
    fn check_budget_exactly_at_limit() {
        let alert = check_budget(10.0, Some(10.0), 0.8, BudgetScope::Task, "1").unwrap();
        assert_eq!(alert.status, BudgetStatus::Exceeded);
    }

    #[test]
    fn check_budget_exactly_at_threshold() {
        let alert = check_budget(8.0, Some(10.0), 0.8, BudgetScope::Task, "1").unwrap();
        assert_eq!(alert.status, BudgetStatus::Warning);
    }

    #[test]
    fn check_task_budgets_no_alerts() {
        let config = BudgetConfig {
            max_cost_per_task: Some(5.0),
            max_cost_per_agent_daily: Some(50.0),
            max_cost_daily: Some(100.0),
            warning_threshold: 0.8,
        };
        let alerts = check_task_budgets(&config, 1.0, 10.0, 20.0, "task-1", "claude-code");
        assert!(alerts.is_empty());
    }

    #[test]
    fn check_task_budgets_task_exceeded() {
        let config = BudgetConfig {
            max_cost_per_task: Some(5.0),
            max_cost_per_agent_daily: Some(50.0),
            max_cost_daily: Some(100.0),
            warning_threshold: 0.8,
        };
        let alerts = check_task_budgets(&config, 6.0, 10.0, 20.0, "task-1", "claude-code");
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].scope, BudgetScope::Task);
        assert_eq!(alerts[0].status, BudgetStatus::Exceeded);
    }

    #[test]
    fn check_task_budgets_multiple_alerts() {
        let config = BudgetConfig {
            max_cost_per_task: Some(5.0),
            max_cost_per_agent_daily: Some(50.0),
            max_cost_daily: Some(100.0),
            warning_threshold: 0.8,
        };
        let alerts = check_task_budgets(&config, 6.0, 55.0, 95.0, "task-1", "claude-code");
        assert_eq!(alerts.len(), 3); // task exceeded, agent exceeded, global warning
    }

    #[test]
    fn budget_scope_display() {
        assert_eq!(BudgetScope::Task.to_string(), "task");
        assert_eq!(BudgetScope::AgentDaily.to_string(), "agent_daily");
        assert_eq!(BudgetScope::GlobalDaily.to_string(), "global_daily");
    }

    #[test]
    fn budget_status_serde_roundtrip() {
        let statuses = vec![
            BudgetStatus::Ok,
            BudgetStatus::Warning,
            BudgetStatus::Exceeded,
            BudgetStatus::Unlimited,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: BudgetStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn budget_alert_serde_roundtrip() {
        let alert = BudgetAlert {
            scope: BudgetScope::Task,
            scope_id: "42".into(),
            status: BudgetStatus::Warning,
            current_cost: 4.5,
            limit: 5.0,
            percentage: 0.9,
            message: "test".into(),
        };
        let json = serde_json::to_string(&alert).unwrap();
        let parsed: BudgetAlert = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.scope, BudgetScope::Task);
        assert_eq!(parsed.scope_id, "42");
    }

    #[test]
    fn budget_config_serde_roundtrip() {
        let config = BudgetConfig {
            max_cost_per_task: Some(5.0),
            max_cost_per_agent_daily: Some(50.0),
            max_cost_daily: Some(200.0),
            warning_threshold: 0.75,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: BudgetConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_cost_per_task, Some(5.0));
        assert!((parsed.warning_threshold - 0.75).abs() < f64::EPSILON);
    }
}
