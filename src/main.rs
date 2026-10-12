use axum::Router;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod http;
mod movies;
mod holds;
mod showtimes;
mod sse;
mod payments;
mod bookings;
mod admin;
mod profiles;

#[derive(Clone)]
pub struct AppState {
    pub config: config::Config,
    pub db: PgPool,
    pub redis: redis::aio::ConnectionManager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Load configuration
    let config = config::Config::from_env();

    // 3. Connect to PostgreSQL
    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(50) // High concurrency pool for Maximum Performance
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to Supabase PostgreSQL");

    tracing::info!("Connecting to redis...");
    let redis_client = redis::Client::open(config.redis_url.clone()).unwrap();
    let redis = redis::aio::ConnectionManager::new(redis_client).await.unwrap();

    // 4. Build application state
    let state = AppState {
        config: config.clone(),
        db: pool,
        redis,
    };
    
    // 5. Define the router
    let app = Router::new()
        .merge(http::api_router(state.clone()))
        .with_state(state);

    // 6. Start the server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
