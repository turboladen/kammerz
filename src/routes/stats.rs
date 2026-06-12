use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::AppState;
use crate::auth::middleware::RequireAuth;
use crate::error::AppResult;
use crate::services::stats_service::{CatalogStats, StatsService};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_stats))
}

async fn get_stats(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<CatalogStats>> {
    Ok(Json(StatsService::get_stats(&db).await?))
}
