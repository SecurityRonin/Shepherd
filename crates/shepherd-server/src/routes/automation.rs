use axum::{extract::Path, extract::State, http::StatusCode, Json};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::state::AppState;
use shepherd_core::automation::AutomationEngine;

#[derive(Deserialize)]
pub struct CreateRuleRequest {
    name: String,
    rule_type: String,
    pattern: String,
    scope: Option<String>,
}

/// GET /api/automation-rules — list all rules.
#[tracing::instrument(skip(state))]
pub async fn list_rules(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let rules = AutomationEngine::list_rules(&db).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::to_value(rules).unwrap()))
}

/// POST /api/automation-rules — create a new rule.
#[tracing::instrument(skip(state, input))]
pub async fn create_rule(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateRuleRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Validate rule_type
    if input.rule_type != "auto_approve" && input.rule_type != "auto_reject" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({ "error": "rule_type must be 'auto_approve' or 'auto_reject'" }),
            ),
        ));
    }

    let db = state.db.lock().await;
    let rule = AutomationEngine::create_rule(
        &db,
        &input.name,
        &input.rule_type,
        &input.pattern,
        input.scope.as_deref(),
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(rule).unwrap()),
    ))
}

/// DELETE /api/automation-rules/:id — delete a rule.
#[tracing::instrument(skip(state))]
pub async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let db = state.db.lock().await;
    let deleted = AutomationEngine::delete_rule(&db, id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}
