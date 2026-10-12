use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{http::error::ApiError, AppState};

// --------------------------------------------------------
// Models
// --------------------------------------------------------
#[derive(Serialize, Deserialize)]
pub struct Genre {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateGenre {
    pub name: String,
    pub slug: String,
}

// --------------------------------------------------------
// Routers
// --------------------------------------------------------
#[derive(Serialize, Deserialize)]
pub struct City {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/genres", get(list_genres).post(create_genre))
        .route("/cities", get(list_cities))
}

async fn list_cities(
    State(state): State<AppState>,
) -> Result<Json<Vec<City>>, ApiError> {
    let cities = sqlx::query_as!(
        City,
        r#"
        SELECT id, name, slug, created_at FROM cities ORDER BY name
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(cities))
}

async fn list_genres(
    State(state): State<AppState>,
) -> Result<Json<Vec<Genre>>, ApiError> {
    let genres = sqlx::query_as!(
        Genre,
        r#"
        SELECT id, name, slug, created_at FROM genres ORDER BY name
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(genres))
}

async fn create_genre(
    State(state): State<AppState>,
    Json(payload): Json<CreateGenre>,
) -> Result<Json<Genre>, ApiError> {
    let genre = sqlx::query_as!(
        Genre,
        r#"
        INSERT INTO genres (name, slug)
        VALUES ($1, $2)
        RETURNING id, name, slug, created_at
        "#,
        payload.name,
        payload.slug
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(genre))
}
