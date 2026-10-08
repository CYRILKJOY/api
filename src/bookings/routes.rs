use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{http::error::ApiError, AppState};

#[derive(Deserialize)]
pub struct CreateBookingRequest {
    pub hold_id: Uuid,
}

#[derive(Serialize)]
pub struct BookingResponse {
    pub id: Uuid,
    pub status: String,
    pub showtime_id: Uuid,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_booking))
        .with_state(state)
}

// Implement Rule 53 & 57: Booking State Machine (HELD -> PAYMENT_PENDING)
async fn create_booking(
    State(state): State<AppState>,
    Json(payload): Json<CreateBookingRequest>,
) -> Result<Json<BookingResponse>, ApiError> {
    let mut tx = state.db.begin().await?;

    let hold = sqlx::query!(
        r#"
        SELECT id, user_id, showtime_id, status::text as "status!", expires_at
        FROM holds
        WHERE id = $1 FOR UPDATE
        "#,
        payload.hold_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::HoldNotFound)?;

    if hold.status != "active" {
        return Err(ApiError::InvalidRequest("Hold is not active".into()));
    }

    if hold.expires_at < chrono::Utc::now() {
        return Err(ApiError::HoldExpired);
    }

    let booking_id = Uuid::new_v4();
    
    // Create Booking in pending_payment state
    sqlx::query!(
        r#"
        INSERT INTO bookings (id, user_id, showtime_id, status)
        VALUES ($1, $2, $3, 'pending_payment')
        "#,
        booking_id,
        hold.user_id,
        hold.showtime_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(BookingResponse {
        id: booking_id,
        status: "pending_payment".into(),
        showtime_id: hold.showtime_id,
    }))
}
