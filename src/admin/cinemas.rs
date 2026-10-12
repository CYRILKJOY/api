use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::{http::error::ApiError, AppState};

#[derive(Serialize, Deserialize)]
pub struct AdminCinema {
    pub id: Uuid,
    pub city_id: Uuid,
    pub name: String,
    pub slug: String,
    pub address: String,
    pub status: String,
    pub image_url: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateAdminCinema {
    pub city_id: Uuid,
    pub name: String,
    pub slug: String,
    pub address: String,
    pub status: String,
    pub image_url: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_cinemas).post(create_cinema))
        .route("/{id}", get(get_cinema).put(update_cinema))
}

async fn get_cinema(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AdminCinema>, ApiError> {
    let cinema = sqlx::query_as!(
        AdminCinema,
        r#"
        SELECT id, city_id, name, slug, address, status, image_url, latitude, longitude, created_at
        FROM cinemas WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Cinema not found".into()))?;

    Ok(Json(cinema))
}

async fn list_cinemas(
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminCinema>>, ApiError> {
    let cinemas = sqlx::query_as!(
        AdminCinema,
        r#"
        SELECT id, city_id, name, slug, address, status, image_url, latitude, longitude, created_at
        FROM cinemas
        ORDER BY name ASC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(cinemas))
}

async fn create_cinema(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdminCinema>,
) -> Result<Json<AdminCinema>, ApiError> {
    let cinema = sqlx::query_as!(
        AdminCinema,
        r#"
        INSERT INTO cinemas (city_id, name, slug, address, status, image_url, latitude, longitude)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, city_id, name, slug, address, status, image_url, latitude, longitude, created_at
        "#,
        payload.city_id,
        payload.name,
        payload.slug,
        payload.address,
        payload.status,
        payload.image_url,
        payload.latitude,
        payload.longitude
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(cinema))
}

async fn update_cinema(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateAdminCinema>,
) -> Result<Json<AdminCinema>, ApiError> {
    let cinema = sqlx::query_as!(
        AdminCinema,
        r#"
        UPDATE cinemas 
        SET city_id = $1, name = $2, slug = $3, address = $4, status = $5, image_url = $6, latitude = $7, longitude = $8
        WHERE id = $9
        RETURNING id, city_id, name, slug, address, status, image_url, latitude, longitude, created_at
        "#,
        payload.city_id,
        payload.name,
        payload.slug,
        payload.address,
        payload.status,
        payload.image_url,
        payload.latitude,
        payload.longitude,
        id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(cinema))
}
