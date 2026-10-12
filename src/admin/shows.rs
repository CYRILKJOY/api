use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{http::error::ApiError, AppState};

#[derive(Serialize, Deserialize)]
pub struct AdminShowtime {
    pub id: Uuid,
    pub movie_id: Uuid,
    pub cinema_id: Uuid,
    pub screen_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateAdminShowtime {
    pub movie_id: Uuid,
    pub cinema_id: Uuid,
    pub screen_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct AdminShowtimePrice {
    pub id: Uuid,
    pub showtime_id: Uuid,
    pub name: String,
    pub price_amount: i32,
}

#[derive(Deserialize)]
pub struct CreateAdminShowtimePrice {
    pub name: String,
    pub price_amount: i32,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_shows).post(create_show))
        .route("/{id}", get(get_show).put(update_show))
        .route("/{id}/prices", get(list_prices).post(add_price))
}

async fn list_shows(
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminShowtime>>, ApiError> {
    let shows = sqlx::query_as!(
        AdminShowtime,
        r#"
        SELECT id, movie_id, cinema_id, screen_id, starts_at, ends_at, status, created_at
        FROM showtimes
        ORDER BY starts_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(shows))
}

async fn create_show(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdminShowtime>,
) -> Result<Json<AdminShowtime>, ApiError> {
    let show = sqlx::query_as!(
        AdminShowtime,
        r#"
        INSERT INTO showtimes (movie_id, cinema_id, screen_id, starts_at, ends_at, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, movie_id, cinema_id, screen_id, starts_at, ends_at, status, created_at
        "#,
        payload.movie_id,
        payload.cinema_id,
        payload.screen_id,
        payload.starts_at,
        payload.ends_at,
        payload.status
    )
    .fetch_one(&state.db)
    .await?;

    // Rule #28: Pre-create showtime_seats for all seats in the screen
    sqlx::query!(
        r#"
        INSERT INTO showtime_seats (showtime_id, seat_id, status, version)
        SELECT $1, id, 'available', 1
        FROM seats WHERE screen_id = $2 AND is_gap = false
        "#,
        show.id,
        show.screen_id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(show))
}

async fn get_show(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AdminShowtime>, ApiError> {
    let show = sqlx::query_as!(
        AdminShowtime,
        r#"
        SELECT id, movie_id, cinema_id, screen_id, starts_at, ends_at, status, created_at
        FROM showtimes WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Showtime not found".into()))?;

    Ok(Json(show))
}

async fn update_show(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateAdminShowtime>,
) -> Result<Json<AdminShowtime>, ApiError> {
    let show = sqlx::query_as!(
        AdminShowtime,
        r#"
        UPDATE showtimes 
        SET movie_id = $1, cinema_id = $2, screen_id = $3, starts_at = $4, ends_at = $5, status = $6
        WHERE id = $7
        RETURNING id, movie_id, cinema_id, screen_id, starts_at, ends_at, status, created_at
        "#,
        payload.movie_id,
        payload.cinema_id,
        payload.screen_id,
        payload.starts_at,
        payload.ends_at,
        payload.status,
        id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(show))
}

async fn list_prices(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<AdminShowtimePrice>>, ApiError> {
    let prices = sqlx::query_as!(
        AdminShowtimePrice,
        r#"
        SELECT id, showtime_id, name, price_amount
        FROM showtime_prices
        WHERE showtime_id = $1
        "#,
        id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(prices))
}

async fn add_price(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateAdminShowtimePrice>,
) -> Result<Json<AdminShowtimePrice>, ApiError> {
    let price = sqlx::query_as!(
        AdminShowtimePrice,
        r#"
        INSERT INTO showtime_prices (showtime_id, name, price_amount)
        VALUES ($1, $2, $3)
        RETURNING id, showtime_id, name, price_amount
        "#,
        id,
        payload.name,
        payload.price_amount
    )
    .fetch_one(&state.db)
    .await?;

    // Here we would typically assign prices to specific seats or seat categories.
    // In a robust CRM, this would be a more complex mapping payload.

    Ok(Json(price))
}
