use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub supabase_jwt_secret: String,
    pub port: u16,
    pub redis_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        // Load .env if it exists
        dotenvy::dotenv().ok();
        
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:54322/postgres".to_string()),
            supabase_jwt_secret: env::var("SUPABASE_JWT_SECRET")
                .unwrap_or_else(|_| "super-secret-jwt-token-with-at-least-32-characters-long".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a valid u16"),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string()),
        }
    }
}
