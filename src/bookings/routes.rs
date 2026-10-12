use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router, Extension,
    middleware::from_fn_with_state,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    http::{error::ApiError, middleware::{require_auth, Claims}},
    AppState,
};

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
        .route("/", post(create_booking).get(list_user_bookings))
        .route("/{id}", get(get_user_booking))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
}

async fn create_booking(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
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

    if hold.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }

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
        INSERT INTO bookings (id, user_id, showtime_id, status, total_amount)
        VALUES ($1, $2, $3, 'pending_payment', 0)
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

async fn list_user_bookings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let bookings = sqlx::query!(
        r#"
        SELECT b.id, b.status::text as status, b.created_at, b.total_amount, m.title as movie_title, c.name as cinema_name
        FROM bookings b
        JOIN showtimes s ON b.showtime_id = s.id
        JOIN movies m ON s.movie_id = m.id
        JOIN cinemas c ON s.cinema_id = c.id
        WHERE b.user_id = $1
        ORDER BY b.created_at DESC
        "#,
        claims.sub
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|row| serde_json::json!({
        "id": row.id,
        "status": row.status,
        "created_at": row.created_at,
        "total_amount": row.total_amount,
        "movie_title": row.movie_title,
        "cinema_name": row.cinema_name
    }))
    .collect();

    Ok(Json(bookings))
}

async fn get_user_booking(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let booking = sqlx::query!(
        r#"
        SELECT b.id, b.user_id, b.status::text as status, b.created_at, b.total_amount, m.title as movie_title, c.name as cinema_name
        FROM bookings b
        JOIN showtimes s ON b.showtime_id = s.id
        JOIN movies m ON s.movie_id = m.id
        JOIN cinemas c ON s.cinema_id = c.id
        WHERE b.id = $1
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Booking not found".into()))?;

    if booking.user_id != claims.sub {
        return Err(ApiError::Forbidden);
    }

    Ok(Json(serde_json::json!({
        "id": booking.id,
        "status": booking.status,
        "created_at": booking.created_at,
        "total_amount": booking.total_amount,
        "movie_title": booking.movie_title,
        "cinema_name": booking.cinema_name
    })))
}
