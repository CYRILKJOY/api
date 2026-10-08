use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{http::error::ApiError, AppState};

#[derive(Serialize, Deserialize)]
pub struct Movie {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub duration_minutes: i32,
    pub poster_url: Option<String>,
}

#[derive(Serialize)]
pub struct ShowtimeResponse {
    pub id: Uuid,
    pub cinema_id: Uuid,
    pub screen_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_movies))
        .route("/{slug}", get(get_movie_by_slug))
        .route("/{id}/showtimes", get(list_movie_showtimes))
}

async fn list_movies(State(state): State<AppState>) -> Result<Json<Vec<Movie>>, ApiError> {
    use redis::AsyncCommands;
    let mut redis_conn = state.redis.clone();

    // Try Cache
    if let Ok(cached) = redis_conn.get::<_, String>("cache:movies:all").await {
        if let Ok(movies) = serde_json::from_str(&cached) {
            return Ok(Json(movies));
        }
    }

    let movies = sqlx::query_as!(
        Movie,
        r#"
        SELECT id, title, slug, duration_minutes, poster_url
        FROM movies
        ORDER BY title ASC
        "#
    )
    .fetch_all(&state.db)
    .await?;

    if let Ok(json) = serde_json::to_string(&movies) {
        let _ = redis_conn.set_ex::<_, _, ()>("cache:movies:all", json, 3600).await;
    }

    Ok(Json(movies))
}

async fn get_movie_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Movie>, ApiError> {
    let movie = sqlx::query_as!(
        Movie,
        r#"
        SELECT id, title, slug, duration_minutes, poster_url
        FROM movies
        WHERE slug = $1
        "#,
        slug
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Movie not found".into()))?;

    Ok(Json(movie))
}

async fn list_movie_showtimes(
    State(state): State<AppState>,
    Path(movie_id): Path<Uuid>,
) -> Result<Json<Vec<ShowtimeResponse>>, ApiError> {
    // Only return future showtimes (Type 2: Cacheable Read fallback to DB)
    let showtimes = sqlx::query_as!(
        ShowtimeResponse,
        r#"
        SELECT id, cinema_id, screen_id, starts_at, ends_at
        FROM showtimes
        WHERE movie_id = $1 AND starts_at > NOW()
        ORDER BY starts_at ASC
        "#,
        movie_id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(showtimes))
}
