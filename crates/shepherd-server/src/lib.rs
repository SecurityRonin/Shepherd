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
        .route("/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
