use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::auth::handlers;
use crate::AppState;

pub mod cameras;
pub mod development;
pub mod film_stocks;
pub mod import;
pub mod labs;
pub mod lens_mounts;
pub mod lenses;
pub mod rolls;
pub mod search;
pub mod settings;
pub mod shots;
pub mod stats;

pub fn create_router(state: AppState) -> Router {
    Router::<AppState>::new()
        .route("/api/health", get(health))
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/logout", post(handlers::logout))
        .route("/api/auth/me", get(handlers::me))
        .nest("/api/cameras", cameras::router())
        .nest("/api/maintenance", cameras::maintenance_router())
        .nest("/api/lenses", lenses::router())
        .nest("/api/lens-mounts", lens_mounts::router())
        .nest("/api/film-stocks", film_stocks::router())
        .nest("/api/labs", labs::router())
        .nest("/api/rolls", rolls::router())
        .nest("/api/shots", shots::router())
        .nest("/api/development", development::router())
        .nest("/api/search", search::router())
        .nest("/api/stats", stats::router())
        .nest("/api/settings", settings::router())
        .nest("/api/import", import::router())
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true }))
}

/// Map a DB error to a user-friendly message. Recognizes common SQLite constraint
/// errors and produces actionable text; falls back to the raw error otherwise.
/// Accepts DbErr, TransactionError<DbErr>, or any Display type.
///
/// The `context` should be a noun phrase (e.g. "roll", "camera", "film stock").
pub fn friendly_err(context: &str, e: impl std::fmt::Display) -> String {
    let raw = e.to_string();

    // SeaORM wraps SQLite errors (e.g. "Execution Error: ... UNIQUE constraint
    // failed: table.col"), so we search with `contains()` + extract the tail.

    // UNIQUE constraint failed: table.column
    if let Some(pos) = raw.find("UNIQUE constraint failed: ") {
        let rest = &raw[pos + "UNIQUE constraint failed: ".len()..];
        // Strip any trailing quote/paren that SeaORM's wrapping may add
        let rest = rest.trim_end_matches(|c: char| c == '"' || c == ')' || c.is_whitespace());
        let col = rest
            .split('.')
            .last()
            .unwrap_or("value")
            .replace('_', " ");
        return format!("A {context} with that {col} already exists.");
    }
    if raw.contains("UNIQUE constraint failed") {
        return format!("A {context} with those values already exists.");
    }

    // FOREIGN KEY constraint failed (usually on delete)
    if raw.contains("FOREIGN KEY constraint failed") {
        return format!(
            "Cannot delete this {context} because it is still referenced by other records."
        );
    }

    // NOT NULL constraint failed: table.column
    if let Some(pos) = raw.find("NOT NULL constraint failed: ") {
        let rest = &raw[pos + "NOT NULL constraint failed: ".len()..];
        let rest = rest.trim_end_matches(|c: char| c == '"' || c == ')' || c.is_whitespace());
        let col = rest
            .split('.')
            .last()
            .unwrap_or("field")
            .replace('_', " ");
        return format!("The {col} field is required.");
    }

    // Default: neutral verb so the message reads correctly with a noun context
    format!("Could not save {context}: {raw}")
}
