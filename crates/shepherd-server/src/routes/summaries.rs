use axum::{extract::Path, http::StatusCode, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct SummaryResponse {
    pub summary: String,
    pub generated_at: String,
}

pub async fn get_task_summary(
    Path(task_id): Path<i64>,
) -> Result<Json<SummaryResponse>, StatusCode> {
    // Stub: return a placeholder summary for now
    // Full LLM integration will come when the LLM module is wired
    Ok(Json(SummaryResponse {
        summary: format!("Task {} completed successfully.", task_id),
        generated_at: chrono::Utc::now().to_rfc3339(),
    }))
}
