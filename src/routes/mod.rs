use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::<AppState>::new()
        .route("/api/health", get(health))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true }))
}
