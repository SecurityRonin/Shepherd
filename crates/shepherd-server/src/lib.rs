pub mod routes;
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
        .route("/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
