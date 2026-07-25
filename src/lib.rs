pub mod activity;
pub mod auth;
pub mod compression;
pub mod config;
pub mod db;
pub mod error;
pub mod extract;
pub mod patch;
pub mod routes;
pub mod services;
pub mod spa;
pub mod validate;

use axum::extract::FromRef;
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: config::AppConfig,
    /// The configured `DATABASE_URL`. Kept so the backup endpoint can open a
    /// short-lived SEPARATE connection for its `VACUUM INTO` snapshot instead of
    /// monopolizing the single (max=min=1) data-pool connection and starving
    /// concurrent `/api/health` probes (kammerz-vlyu.16).
    pub db_url: String,
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
