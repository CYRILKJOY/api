use axum::{Router, middleware::from_fn_with_state};
use crate::{AppState, http::middleware::require_admin};

pub mod configuration;
pub mod movies;
pub mod cinemas;
pub mod customers;
pub mod screens;
pub mod shows;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/configuration", configuration::router(state.clone()))
        .nest("/movies", movies::router(state.clone()))
        .nest("/cinemas", cinemas::router(state.clone()))
        .nest("/customers", customers::router(state.clone()))
        .merge(screens::router(state.clone()))
        .nest("/shows", shows::router(state.clone()))
        // Apply the require_admin middleware to ALL routes in the admin namespace
        .route_layer(from_fn_with_state(state, require_admin))
}
