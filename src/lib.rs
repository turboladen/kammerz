pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod patch;
pub mod routes;
pub mod services;

use std::sync::Arc;

use axum::extract::FromRef;
use sea_orm::DatabaseConnection;

use auth::rate_limit::LoginRateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: config::AppConfig,
    /// Per-IP failed-login throttle, shared across all handler clones.
    pub login_rate_limiter: Arc<LoginRateLimiter>,
}

impl AppState {
    /// Build state with a fresh login throttle. Each server (and each test app)
    /// gets its own limiter, so failure budgets never leak across instances.
    pub fn new(db: DatabaseConnection, config: config::AppConfig) -> Self {
        Self {
            db,
            config,
            login_rate_limiter: Arc::new(LoginRateLimiter::default()),
        }
    }
}

// Lets handlers extract `State<DatabaseConnection>` directly (chorez pattern).
impl FromRef<AppState> for DatabaseConnection {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for config::AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for Arc<LoginRateLimiter> {
    fn from_ref(state: &AppState) -> Self {
        state.login_rate_limiter.clone()
    }
}
