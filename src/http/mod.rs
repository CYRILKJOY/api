pub mod error;
pub mod middleware;

use axum::{Router, routing::get, Json};
use serde_json::{json, Value};

use crate::AppState;

// This will later register domains like `movies`, `holds`
pub fn api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1/movies", crate::movies::routes::router())
        .nest("/api/v1/holds", crate::holds::routes::router(state.clone()))
        .nest("/api/v1/showtimes", crate::showtimes::routes::router())
        .nest("/api/v1/sse", crate::sse::routes::router())
        .nest("/api/v1/payments", crate::payments::routes::router(state.clone()))
        .nest("/api/v1/bookings", crate::bookings::routes::router(state.clone()))
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "maximum-performance-booking-api"
    }))
}
