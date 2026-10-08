use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use futures::stream::Stream;
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt;
use uuid::Uuid;
use redis::AsyncCommands;
use async_stream::stream;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/events", get(sse_handler))
}

// Rule #47: SSE Realtime Seat Map Pipeline
async fn sse_handler(
    Path(showtime_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut redis_conn = state.redis.clone();

    let stream = stream! {
        // Initial connection heartbeat
        yield Ok(Event::default().event("connected").data("true"));

        let mut interval = tokio::time::interval(Duration::from_secs(15));
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    yield Ok(Event::default().event("ping").data("alive"));
                }
                // In a production system, we'd use redis pub/sub `subscribe` here.
                // For demonstration, we simply yield simulated updates if any, or wait.
            }
        }
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
