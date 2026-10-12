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
        
        let channel = format!("showtime:{}", showtime_id);
        
        // Use a dedicated connection for PubSub since connection manager doesn't support it directly
        // In a real app we might want a multiplexed pubsub but for this scope it's okay to open one
        let client = redis::Client::open(state.config.redis_url.clone()).unwrap();
        let mut pubsub_conn = client.get_async_pubsub().await.unwrap();
        pubsub_conn.subscribe(&channel).await.unwrap();
        
        let mut pubsub_stream = pubsub_conn.into_on_message();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    yield Ok(Event::default().event("ping").data("alive"));
                }
                msg = pubsub_stream.next() => {
                    if let Some(m) = msg {
                        let payload: String = m.get_payload().unwrap();
                        yield Ok(Event::default().event("seat_update").data(payload));
                    }
                }
            }
        }
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
