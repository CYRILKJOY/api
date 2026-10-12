use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

use crate::{http::error::ApiError, AppState};

#[derive(Serialize, Deserialize)]
pub struct AdminMovie {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub duration_minutes: i32,
    pub poster_url: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub trailer_url: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub age_rating: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateAdminMovie {
    pub title: String,
    pub slug: String,
    pub duration_minutes: i32,
    pub status: String,
    pub description: Option<String>,
    pub poster_url: Option<String>,
    pub trailer_url: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub age_rating: Option<String>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_movies).post(create_movie))
        .route("/{id}", get(get_movie).put(update_movie))
}

async fn get_movie(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AdminMovie>, ApiError> {
    let movie = sqlx::query_as!(
        AdminMovie,
        r#"
        SELECT id, title, slug, duration_minutes, poster_url, status, description, trailer_url, release_date, age_rating, created_at
        FROM movies WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Movie not found".into()))?;

    Ok(Json(movie))
}

async fn list_movies(
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminMovie>>, ApiError> {
    let movies = sqlx::query_as!(
        AdminMovie,
        r#"
        SELECT id, title, slug, duration_minutes, poster_url, status, description, trailer_url, release_date, age_rating, created_at
        FROM movies
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(movies))
}

async fn create_movie(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdminMovie>,
) -> Result<Json<AdminMovie>, ApiError> {
    let movie = sqlx::query_as!(
        AdminMovie,
        r#"
        INSERT INTO movies (title, slug, duration_minutes, status, description, poster_url, trailer_url, release_date, age_rating)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, title, slug, duration_minutes, poster_url, status, description, trailer_url, release_date, age_rating, created_at
        "#,
        payload.title,
        payload.slug,
        payload.duration_minutes,
        payload.status,
        payload.description,
        payload.poster_url,
        payload.trailer_url,
        payload.release_date,
        payload.age_rating
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(movie))
}

async fn update_movie(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateAdminMovie>,
) -> Result<Json<AdminMovie>, ApiError> {
    let movie = sqlx::query_as!(
        AdminMovie,
        r#"
        UPDATE movies 
        SET title = $1, slug = $2, duration_minutes = $3, status = $4, description = $5, poster_url = $6, trailer_url = $7, release_date = $8, age_rating = $9
        WHERE id = $10
        RETURNING id, title, slug, duration_minutes, poster_url, status, description, trailer_url, release_date, age_rating, created_at
        "#,
        payload.title,
        payload.slug,
        payload.duration_minutes,
        payload.status,
        payload.description,
        payload.poster_url,
        payload.trailer_url,
        payload.release_date,
        payload.age_rating,
        id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(movie))
}
