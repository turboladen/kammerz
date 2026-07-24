use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde::Deserialize;

use crate::AppState;
use crate::auth::middleware::RequireAuth;
use crate::error::AppResult;
use crate::extract::{Json, Query};
use crate::services::search_service::{SearchResults, SearchService};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(search))
}

async fn search(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Query(params): Query<SearchQuery>,
) -> AppResult<Json<SearchResults>> {
    let query = params.q.trim();
    // Return empty results for very short queries (parity with the old command).
    // Count characters, not bytes — a single multibyte glyph must not clear the
    // 2-char floor (kammerz-vlyu.27).
    if query.chars().count() < 2 {
        return Ok(Json(SearchResults {
            cameras: vec![],
            lenses: vec![],
            film_stocks: vec![],
            rolls: vec![],
            shots: vec![],
            labs: vec![],
        }));
    }
    Ok(Json(SearchService::search(&db, query).await?))
}
