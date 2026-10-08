use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{http::error::ApiError, AppState};

#[derive(Serialize)]
pub struct SeatResponse {
    pub id: Uuid,
    pub showtime_id: Uuid,
    pub row_label: String,
    pub seat_number: String,
    pub status: String,
    pub price_amount: Option<i32>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/seats", get(get_showtime_seats))
}

async fn get_showtime_seats(
    State(state): State<AppState>,
    Path(showtime_id): Path<Uuid>,
) -> Result<Json<Vec<SeatResponse>>, ApiError> {
    // Type 3: Live Read (Rule #4)
    // Directly queries PostgreSQL to get the most authoritative state of the seats.
    let seats = sqlx::query_as!(
        SeatResponse,
        r#"
        SELECT 
            ss.seat_id as id, 
            ss.showtime_id, 
            s.row_label, 
            s.seat_number, 
            ss.status::text as "status!", 
            sp.price_amount
        FROM showtime_seats ss
        JOIN seats s ON ss.seat_id = s.id
        LEFT JOIN showtime_prices sp ON ss.price_id = sp.id
        WHERE ss.showtime_id = $1
        ORDER BY s.row_label ASC, CAST(s.seat_number AS INTEGER) ASC
        "#,
        showtime_id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(seats))
}
