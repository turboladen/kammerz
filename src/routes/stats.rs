use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::services::stats_service::{CatalogStats, StatsService};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_stats))
}

async fn get_stats(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<CatalogStats>> {
    StatsService::get_stats(&db)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}
