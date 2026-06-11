pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod patch;
pub mod routes;
pub mod services;
pub mod validate;

use axum::extract::FromRef;
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: config::AppConfig,
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
