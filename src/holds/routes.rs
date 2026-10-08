use axum::{
    extract::State,
    routing::post,
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

#[derive(Deserialize)]
pub struct CreateHoldRequest {
    pub showtime_id: Uuid,
    pub seat_ids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct HoldResponse {
    pub success: bool,
    pub hold_id: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(create_hold))
        // Protect all hold routes with our Supabase JWT middleware (Rule #16 & #18)
        .route_layer(from_fn_with_state(state, require_auth))
}

async fn create_hold(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateHoldRequest>,
) -> Result<Json<HoldResponse>, ApiError> {
    
    // Invoke the atomic PostgreSQL function we created earlier (Rule #32, #34)
    // This executes entirely on the database side for maximum safety and minimum latency.
    let record = sqlx::query!(
        r#"
        SELECT create_hold($1, $2, $3, $4) as result
        "#,
        claims.sub, // The trusted User ID from JWT
        payload.showtime_id,
        &payload.seat_ids,
        5 // 5 minutes expiration
    )
    .fetch_one(&state.db)
    .await?;

    // The function returns JSON, we parse it into our Response struct
    let result_json = record.result.ok_or_else(|| ApiError::Internal(anyhow::anyhow!("No result from create_hold")))?;
    
    let response: HoldResponse = serde_json::from_value(result_json)
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("Failed to parse hold result")))?;

    if !response.success {
        return Err(ApiError::SeatsUnavailable);
    }

    Ok(Json(response))
}
