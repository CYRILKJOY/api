use axum::{
    extract::State,
    routing::get,
    Json, Router, Extension,
    middleware::from_fn_with_state,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{
    http::{error::ApiError, middleware::{require_auth, Claims}},
    AppState,
};

#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub full_name: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(get_my_profile))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
}

async fn get_my_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserProfile>, ApiError> {
    let profile = sqlx::query_as!(
        UserProfile,
        r#"
        SELECT id, email, full_name, role, created_at
        FROM profiles
        WHERE id = $1
        "#,
        claims.sub
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Profile not found".into()))?;

    Ok(Json(profile))
}
