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
pub struct AdminScreen {
    pub id: Uuid,
    pub cinema_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateAdminScreen {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct SeatLayoutNode {
    pub id: Option<Uuid>,
    pub row_label: String,
    pub seat_number: String,
    pub grid_row: Option<i32>,
    pub grid_col: Option<i32>,
    pub seat_category: String,
    pub is_gap: bool,
}

#[derive(Deserialize)]
pub struct SaveLayoutPayload {
    pub seats: Vec<SeatLayoutNode>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/cinema/{cinema_id}/screens", get(list_screens).post(create_screen))
        .route("/screens/{id}", get(get_screen).put(update_screen))
        .route("/screens/{id}/layout", get(get_layout).put(save_layout))
}

async fn list_screens(
    State(state): State<AppState>,
    Path(cinema_id): Path<Uuid>,
) -> Result<Json<Vec<AdminScreen>>, ApiError> {
    let screens = sqlx::query_as!(
        AdminScreen,
        r#"
        SELECT id, cinema_id, name, created_at
        FROM screens
        WHERE cinema_id = $1
        ORDER BY name ASC
        "#,
        cinema_id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(screens))
}

async fn create_screen(
    State(state): State<AppState>,
    Path(cinema_id): Path<Uuid>,
    Json(payload): Json<CreateAdminScreen>,
) -> Result<Json<AdminScreen>, ApiError> {
    let screen = sqlx::query_as!(
        AdminScreen,
        r#"
        INSERT INTO screens (cinema_id, name)
        VALUES ($1, $2)
        RETURNING id, cinema_id, name, created_at
        "#,
        cinema_id,
        payload.name
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(screen))
}

async fn get_screen(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AdminScreen>, ApiError> {
    let screen = sqlx::query_as!(
        AdminScreen,
        r#"
        SELECT id, cinema_id, name, created_at
        FROM screens WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Screen not found".into()))?;

    Ok(Json(screen))
}

async fn update_screen(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateAdminScreen>,
) -> Result<Json<AdminScreen>, ApiError> {
    let screen = sqlx::query_as!(
        AdminScreen,
        r#"
        UPDATE screens SET name = $1 WHERE id = $2
        RETURNING id, cinema_id, name, created_at
        "#,
        payload.name,
        id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(screen))
}

async fn get_layout(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SeatLayoutNode>>, ApiError> {
    let seats = sqlx::query_as!(
        SeatLayoutNode,
        r#"
        SELECT id, row_label, seat_number, grid_row, grid_col, seat_category, is_gap
        FROM seats
        WHERE screen_id = $1
        ORDER BY grid_row, grid_col
        "#,
        id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(seats))
}

async fn save_layout(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<SaveLayoutPayload>,
) -> Result<Json<()>, ApiError> {
    let mut tx = state.db.begin().await?;

    // Clear existing layout
    sqlx::query!("DELETE FROM seats WHERE screen_id = $1", id)
        .execute(&mut *tx)
        .await?;

    // Insert new layout
    for seat in payload.seats {
        sqlx::query!(
            r#"
            INSERT INTO seats (screen_id, row_label, seat_number, grid_row, grid_col, seat_category, is_gap)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            id,
            seat.row_label,
            seat.seat_number,
            seat.grid_row,
            seat.grid_col,
            seat.seat_category,
            seat.is_gap
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(()))
}
