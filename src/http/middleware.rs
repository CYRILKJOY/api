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
    validation.validate_nbf = false; // Supabase doesn't always set this
    // Supabase JWT uses HS256 algorithm by default

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

    // Insert the decoded Claims into the request extensions
    // Rust handlers will extract this using `Extension<Claims>`
    req.extensions_mut().insert(token_data.claims);

    Ok(next.run(req).await)
}
