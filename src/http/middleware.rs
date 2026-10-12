use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid, // Supabase Auth User ID (Authoritative Identity)
    pub role: String,
    pub exp: usize,
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = extract_claims(&state, &req)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

pub async fn require_admin(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = extract_claims(&state, &req)?;

    // Verify admin role in the database for security
    let profile = sqlx::query!(
        r#"
        SELECT role FROM profiles WHERE id = $1
        "#,
        claims.sub
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let is_admin = match profile {
        Some(p) => p.role == "admin",
        None => false,
    };

    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

fn extract_claims(state: &AppState, req: &Request) -> Result<Claims, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation.validate_nbf = false; 

    let token_data = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.supabase_jwt_secret.as_bytes()),
        &validation,
    ) {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!("JWT validation failed: {}", err);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    Ok(token_data.claims)
}
