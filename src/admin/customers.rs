use axum::{
    extract::{Path, Query, State},
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{http::error::ApiError, AppState};

#[derive(Serialize, Deserialize)]
pub struct CustomerProfile {
    pub id: Uuid,
    pub email: String,
    pub full_name: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CustomerPagination {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
}

#[derive(Serialize)]
pub struct PaginatedCustomers {
    pub data: Vec<CustomerProfile>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

#[derive(Serialize)]
pub struct CustomerBooking {
    pub id: Uuid,
    pub showtime_id: Uuid,
    pub total_amount: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub movie_title: String,
    pub cinema_name: String,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_customers))
        .route("/{id}", get(get_customer).put(update_customer))
        .route("/{id}/bookings", get(list_customer_bookings))
}

async fn list_customers(
    State(state): State<AppState>,
    Query(params): Query<CustomerPagination>,
) -> Result<Json<PaginatedCustomers>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    let search = params.search.unwrap_or_default();
    let search_pattern = format!("%{}%", search);

    // Get total count
    let total_record = sqlx::query!(
        r#"
        SELECT COUNT(*) as "count!" FROM profiles
        WHERE email ILIKE $1 OR COALESCE(full_name, '') ILIKE $1
        "#,
        search_pattern
    )
    .fetch_one(&state.db)
    .await?;

    let customers = sqlx::query_as!(
        CustomerProfile,
        r#"
        SELECT id, email, full_name, role, created_at
        FROM profiles
        WHERE email ILIKE $1 OR COALESCE(full_name, '') ILIKE $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        search_pattern,
        limit,
        offset
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(PaginatedCustomers {
        data: customers,
        total: total_record.count,
        page,
        limit,
    }))
}

async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CustomerProfile>, ApiError> {
    let customer = sqlx::query_as!(
        CustomerProfile,
        r#"
        SELECT id, email, full_name, role, created_at
        FROM profiles
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Customer not found".into()))?;

    Ok(Json(customer))
}

#[derive(Deserialize)]
pub struct UpdateCustomer {
    pub role: String,
}

async fn update_customer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCustomer>,
) -> Result<Json<CustomerProfile>, ApiError> {
    let customer = sqlx::query_as!(
        CustomerProfile,
        r#"
        UPDATE profiles SET role = $1 WHERE id = $2
        RETURNING id, email, full_name, role, created_at
        "#,
        payload.role,
        id
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(customer))
}

async fn list_customer_bookings(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<CustomerBooking>>, ApiError> {
    let bookings = sqlx::query_as!(
        CustomerBooking,
        r#"
        SELECT b.id, b.showtime_id, b.total_amount, b.status::text as "status!", b.created_at,
               m.title as movie_title, c.name as cinema_name
        FROM bookings b
        JOIN showtimes s ON b.showtime_id = s.id
        JOIN movies m ON s.movie_id = m.id
        JOIN cinemas c ON s.cinema_id = c.id
        WHERE b.user_id = $1
        ORDER BY b.created_at DESC
        "#,
        id
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(bookings))
}
