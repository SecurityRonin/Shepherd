use serde::{Deserialize, Serialize};
use super::pricing;

/// Cost estimate for a single LLM call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub model_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub estimated_cost_usd: f64,
}

impl CostEstimate {
    /// Create a cost estimate from token usage and model ID.
    pub fn from_usage(model_id: &str, input_tokens: u32, output_tokens: u32) -> Self {
        let cost = pricing::estimate_cost(model_id, input_tokens, output_tokens);
        Self {
            model_id: model_id.to_string(),
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost_usd: cost,
        }
    }
}

/// Aggregated metrics for a task (across all LLM calls in a session).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub task_id: i64,
    pub agent_id: String,
    pub model_id: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub llm_calls: u32,
    pub duration_secs: Option<f64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Accumulator for building TaskMetrics incrementally.
#[derive(Debug, Clone)]
pub struct MetricsAccumulator {
    pub task_id: i64,
    pub agent_id: String,
    pub model_id: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub llm_calls: u32,
    start_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl MetricsAccumulator {
    /// Start a new accumulator for a task.
    pub fn new(task_id: i64, agent_id: &str) -> Self {
        Self {
            task_id,
            agent_id: agent_id.to_string(),
            model_id: String::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
            llm_calls: 0,
            start_time: Some(chrono::Utc::now()),
        }
    }

    /// Record a single LLM call's token usage.
    pub fn record(&mut self, model_id: &str, input_tokens: u32, output_tokens: u32) {
        let cost = CostEstimate::from_usage(model_id, input_tokens, output_tokens);
        self.total_input_tokens += input_tokens as u64;
        self.total_output_tokens += output_tokens as u64;
        self.total_cost_usd += cost.estimated_cost_usd;
        self.llm_calls += 1;
        if self.model_id.is_empty() {
            self.model_id = model_id.to_string();
        }
    }

    /// Get the elapsed duration since the accumulator was created.
    pub fn elapsed_secs(&self) -> Option<f64> {
        self.start_time.map(|start| {
            let elapsed = chrono::Utc::now() - start;
            elapsed.num_milliseconds() as f64 / 1000.0
        })
    }

    /// Finalize into a TaskMetrics snapshot.
    pub fn finalize(&self, status: &str) -> TaskMetrics {
        let now = chrono::Utc::now().to_rfc3339();
        TaskMetrics {
            task_id: self.task_id,
            agent_id: self.agent_id.clone(),
            model_id: self.model_id.clone(),
            total_input_tokens: self.total_input_tokens,
            total_output_tokens: self.total_output_tokens,
            total_tokens: self.total_input_tokens + self.total_output_tokens,
            total_cost_usd: self.total_cost_usd,
            llm_calls: self.llm_calls,
            duration_secs: self.elapsed_secs(),
            status: status.to_string(),
            created_at: self.start_time
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| now.clone()),
            updated_at: now,
        }
    }
}

/// Summary of spending across multiple tasks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpendingSummary {
    pub total_cost_usd: f64,
    pub total_tokens: u64,
    pub total_tasks: u32,
    pub total_llm_calls: u32,
    /// Cost breakdown by agent.
    pub by_agent: Vec<AgentSpending>,
    /// Cost breakdown by model.
    pub by_model: Vec<ModelSpending>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpending {
    pub agent_id: String,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
    pub task_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpending {
    pub model_id: String,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
    pub call_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_estimate_from_known_model() {
        let est = CostEstimate::from_usage("claude-sonnet-4", 10_000, 5_000);
        assert_eq!(est.total_tokens, 15_000);
        assert!(est.estimated_cost_usd > 0.0);
        // $3/M * 0.01 + $15/M * 0.005 = $0.03 + $0.075 = $0.105
        assert!((est.estimated_cost_usd - 0.105).abs() < 1e-10);
    }

    #[test]
    fn cost_estimate_unknown_model_zero_cost() {
        let est = CostEstimate::from_usage("unknown-model", 10_000, 5_000);
        assert_eq!(est.total_tokens, 15_000);
        assert!((est.estimated_cost_usd).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_estimate_serde_roundtrip() {
        let est = CostEstimate::from_usage("gpt-4o", 1000, 500);
        let json = serde_json::to_string(&est).unwrap();
        let parsed: CostEstimate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model_id, "gpt-4o");
        assert_eq!(parsed.input_tokens, 1000);
    }

    #[test]
    fn accumulator_records_usage() {
        let mut acc = MetricsAccumulator::new(1, "claude-code");
        acc.record("claude-sonnet-4", 5_000, 2_000);
        acc.record("claude-sonnet-4", 3_000, 1_000);

        assert_eq!(acc.total_input_tokens, 8_000);
        assert_eq!(acc.total_output_tokens, 3_000);
        assert_eq!(acc.llm_calls, 2);
        assert!(acc.total_cost_usd > 0.0);
        assert_eq!(acc.model_id, "claude-sonnet-4");
    }

    #[test]
    fn accumulator_finalize() {
        let mut acc = MetricsAccumulator::new(42, "codex");
        acc.record("gpt-4o", 10_000, 5_000);

        let metrics = acc.finalize("done");
        assert_eq!(metrics.task_id, 42);
        assert_eq!(metrics.agent_id, "codex");
        assert_eq!(metrics.total_input_tokens, 10_000);
        assert_eq!(metrics.total_output_tokens, 5_000);
        assert_eq!(metrics.total_tokens, 15_000);
        assert_eq!(metrics.llm_calls, 1);
        assert_eq!(metrics.status, "done");
        assert!(metrics.duration_secs.is_some());
        assert!(!metrics.created_at.is_empty());
    }

    #[test]
    fn accumulator_empty_finalize() {
        let acc = MetricsAccumulator::new(1, "claude-code");
        let metrics = acc.finalize("queued");
        assert_eq!(metrics.total_tokens, 0);
        assert_eq!(metrics.llm_calls, 0);
        assert!((metrics.total_cost_usd).abs() < f64::EPSILON);
    }

    #[test]
    fn accumulator_elapsed_secs() {
        let acc = MetricsAccumulator::new(1, "claude-code");
        let elapsed = acc.elapsed_secs().unwrap();
        assert!(elapsed >= 0.0);
        assert!(elapsed < 1.0); // Should be nearly instant
    }

    #[test]
    fn task_metrics_serde_roundtrip() {
        let mut acc = MetricsAccumulator::new(1, "claude-code");
        acc.record("claude-sonnet-4", 1000, 500);
        let metrics = acc.finalize("done");

        let json = serde_json::to_string(&metrics).unwrap();
        let parsed: TaskMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.task_id, 1);
        assert_eq!(parsed.agent_id, "claude-code");
    }

    #[test]
    fn spending_summary_default() {
        let summary = SpendingSummary::default();
        assert!((summary.total_cost_usd).abs() < f64::EPSILON);
        assert_eq!(summary.total_tasks, 0);
    }

    #[test]
    fn spending_summary_clone() {
        let mut summary = SpendingSummary::default();
        summary.total_cost_usd = 3.14;
        summary.total_tasks = 7;
        let cloned = summary.clone();
        assert!((cloned.total_cost_usd - 3.14).abs() < f64::EPSILON);
        assert_eq!(cloned.total_tasks, 7);
    }

    #[test]
    fn spending_summary_serde_roundtrip() {
        let summary = SpendingSummary {
            total_cost_usd: 1.23,
            total_tokens: 50_000,
            total_tasks: 5,
            total_llm_calls: 15,
            by_agent: vec![],
            by_model: vec![],
        };
        let json = serde_json::to_string(&summary).unwrap();
        let parsed: SpendingSummary = serde_json::from_str(&json).unwrap();
        assert!((parsed.total_cost_usd - 1.23).abs() < f64::EPSILON);
        assert_eq!(parsed.total_tasks, 5);
    }

    #[test]
    fn accumulator_model_id_set_on_first_record() {
        let mut acc = MetricsAccumulator::new(1, "agent");
        assert!(acc.model_id.is_empty());
        acc.record("gpt-4o", 1000, 500);
        assert_eq!(acc.model_id, "gpt-4o");
        // Second record with different model does not change model_id
        acc.record("claude-sonnet-4", 1000, 500);
        assert_eq!(acc.model_id, "gpt-4o");
    }

    #[test]
    fn accumulator_clone() {
        let mut acc = MetricsAccumulator::new(5, "codex");
        acc.record("gpt-4o", 2000, 1000);
        let cloned = acc.clone();
        assert_eq!(cloned.task_id, 5);
        assert_eq!(cloned.llm_calls, 1);
        assert_eq!(cloned.total_input_tokens, 2000);
    }

    #[test]
    fn cost_estimate_total_tokens_is_sum() {
        let est = CostEstimate::from_usage("claude-sonnet-4", 3000, 7000);
        assert_eq!(est.total_tokens, 10_000);
        assert_eq!(est.input_tokens, 3000);
        assert_eq!(est.output_tokens, 7000);
    }
}
