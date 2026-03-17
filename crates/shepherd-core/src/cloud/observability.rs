use serde::{Deserialize, Serialize};
use super::CloudClient;

#[derive(Debug, Clone, Serialize)]
pub struct MetricsPushPayload {
    pub machine_id: String,
    pub metrics: Vec<MetricEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricEntry {
    pub task_id: i64,
    pub agent_id: String,
    pub model_id: String,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub llm_calls: u32,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudSpendingSummary {
    pub total_cost_usd: f64,
    pub total_tokens: u64,
    pub total_tasks: u32,
    pub by_machine: Vec<MachineSpending>,
    pub by_agent: Vec<CloudAgentSpending>,
    pub by_model: Vec<CloudModelSpending>,
    pub daily_costs: Vec<DailyCost>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineSpending {
    pub machine_id: String,
    pub total_cost_usd: f64,
    pub task_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudAgentSpending {
    pub agent_id: String,
    pub total_cost_usd: f64,
    pub task_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloudModelSpending {
    pub model_id: String,
    pub total_cost_usd: f64,
    pub call_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DailyCost {
    pub date: String,
    pub cost_usd: f64,
}

impl CloudClient {
    pub async fn push_metrics(&self, payload: &MetricsPushPayload) -> Result<(), super::CloudError> {
        let jwt = self.get_jwt()?;
        let url = format!("{}/api/observability/push", self.api_url());
        let resp = self.http.post(&url)
            .bearer_auth(&jwt)
            .json(payload)
            .send()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(super::CloudError::Api { status, message: body });
        }
        Ok(())
    }

    pub async fn cloud_spending_summary(&self, days: u32) -> Result<CloudSpendingSummary, super::CloudError> {
        let jwt = self.get_jwt()?;
        let url = format!("{}/api/observability/summary?days={}", self.api_url(), days);
        let resp = self.http.get(&url)
            .bearer_auth(&jwt)
            .send()
            .await
            .map_err(|e| super::CloudError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(super::CloudError::Api { status, message: body });
        }

        resp.json().await
            .map_err(|e| super::CloudError::Network(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_push_payload_serializes() {
        let payload = MetricsPushPayload {
            machine_id: "mbp-2024".to_string(),
            metrics: vec![MetricEntry {
                task_id: 1,
                agent_id: "claude-code".to_string(),
                model_id: "claude-sonnet-4".to_string(),
                total_tokens: 15000,
                total_cost_usd: 0.50,
                llm_calls: 3,
                status: "done".to_string(),
                created_at: "2026-03-17T00:00:00Z".to_string(),
            }],
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("mbp-2024"));
        assert!(json.contains("claude-code"));
    }

    #[test]
    fn cloud_spending_summary_deserializes() {
        let json = r#"{
            "total_cost_usd": 2.50,
            "total_tokens": 50000,
            "total_tasks": 5,
            "by_machine": [{"machine_id":"mbp","total_cost_usd":1.5,"task_count":3}],
            "by_agent": [{"agent_id":"claude-code","total_cost_usd":2.5,"task_count":5}],
            "by_model": [{"model_id":"claude-sonnet-4","total_cost_usd":2.5,"call_count":15}],
            "daily_costs": [{"date":"2026-03-17","cost_usd":2.5}]
        }"#;
        let summary: CloudSpendingSummary = serde_json::from_str(json).unwrap();
        assert_eq!(summary.total_tasks, 5);
        assert_eq!(summary.by_machine.len(), 1);
        assert_eq!(summary.daily_costs.len(), 1);
    }

    #[test]
    fn daily_cost_deserializes() {
        let json = r#"{"date":"2026-03-17","cost_usd":1.23}"#;
        let cost: DailyCost = serde_json::from_str(json).unwrap();
        assert_eq!(cost.date, "2026-03-17");
        assert!((cost.cost_usd - 1.23).abs() < f64::EPSILON);
    }

    #[test]
    fn empty_metrics_payload() {
        let payload = MetricsPushPayload {
            machine_id: "test".to_string(),
            metrics: vec![],
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"metrics\":[]"));
    }
}
