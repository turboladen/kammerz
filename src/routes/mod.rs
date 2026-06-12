use axum::routing::{get, post};
use axum::{Json, Router};
use sea_orm::{DbErr, TransactionError};
use serde_json::{json, Value};
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;

use crate::auth::{handlers, rate_limit};
use crate::error::AppError;
use crate::AppState;

pub mod backup;
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
    // Per-IP brute-force guard, scoped to the login route only via `.layer()` on
    // its MethodRouter (logout/me/business routes are untouched). See
    // `auth::rate_limit` for the rationale and tuning constants.
    let login_rate_limit = GovernorLayer::new(
        GovernorConfigBuilder::default()
            .burst_size(rate_limit::LOGIN_BURST_SIZE)
            .per_second(rate_limit::LOGIN_REPLENISH_SECONDS)
            .finish()
            .expect("login rate-limit config is valid"),
    )
    .error_handler(rate_limit::on_governor_error);

    Router::<AppState>::new()
        .route("/api/health", get(health))
        .route(
            "/api/auth/login",
            post(handlers::login).layer(login_rate_limit),
        )
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
        .nest("/api/backup", backup::router())
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

/// Classify a transaction error into the right HTTP status. A not-found lookup
/// inside the closure — `DbErr::RecordNotFound`, produced by
/// [`crate::error::DbOptionExt::or_404_db`] — becomes a 404; a `DbErr::Custom`
/// (an already-friendly business-rule rejection raised inside the closure, e.g.
/// the lab/self dev mutual-exclusion guard) passes through verbatim as a 422;
/// every other error stays a friendly 422. The transactional delete handlers
/// (shots, lab/self dev) use this so a double-delete of a stale id returns
/// NOT_FOUND, matching the non-transactional handlers' `or_404`. The inner
/// messages are taken directly (not via `Display`) to avoid SeaORM's
/// "RecordNotFound Error: " / "Custom Error: " prefixes.
pub fn friendly_txn_err(context: &str, e: TransactionError<DbErr>) -> AppError {
    match e {
        TransactionError::Transaction(DbErr::RecordNotFound(m)) => AppError::NotFound(m),
        TransactionError::Transaction(DbErr::Custom(m)) => AppError::UnprocessableEntity(m),
        other => AppError::UnprocessableEntity(friendly_err(context, other)),
    }
}
