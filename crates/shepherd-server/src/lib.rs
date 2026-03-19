pub mod routes;
pub mod startup;
pub mod state;
pub mod ws;

use axum::{
    routing::{delete, get, post},
    Router,
};
use state::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(routes::health::health))
        .route("/api/tasks", get(routes::tasks::list_tasks))
        .route("/api/tasks", post(routes::tasks::create_task))
        .route("/api/tasks/:id", delete(routes::tasks::delete_task))
        .route("/api/namegen", post(routes::namegen::generate_names))
        .route("/api/logogen", post(routes::logogen::generate_logo))
        .route("/api/logogen/export", post(routes::logogen::export_icons))
        .route("/api/northstar/phases", get(routes::northstar::list_phases))
        .route("/api/northstar/phase", post(routes::northstar::execute_phase))
        .route("/api/tasks/:id/gates", post(routes::gates::run_task_gates))
        .route("/api/tasks/:id/pr", post(routes::pr::create_pr))
        .route("/api/sessions/:id/claude-sessions", get(routes::iterm2::list_claude_sessions))
        .route("/api/sessions/:id/resume", post(routes::iterm2::resume_session))
        .route("/api/sessions/:id/fresh", post(routes::iterm2::fresh_session))
        .route("/api/cloud/status", get(routes::cloud::cloud_status))
        .route("/api/cloud/balance", get(routes::cloud::cloud_balance))
        .route("/api/cloud/costs", get(routes::cloud::cloud_costs))
        .route("/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
mod handler_tests;
