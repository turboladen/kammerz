//! Shared test helpers for integration tests.
//!
//! `mod common;` is compiled independently into each test binary, so any helper
//! a given test file doesn't use shows as dead code there. Allow it module-wide.
#![allow(dead_code)]

use axum::body::Body;
use axum::http::Request;
use axum::response::Response;
use http_body_util::BodyExt;
use serde::Serialize;
use serde::de::DeserializeOwned;

use kammerz::AppState;
use kammerz::config::AppConfig;

/// Build an app in OPEN auth mode (no password) backed by a fresh in-memory DB.
/// In open mode `RequireAuth` passes without a session, so no session layer is needed.
pub async fn open_app() -> axum::Router {
    open_app_with_url("sqlite::memory:").await
}

/// Like [`open_app`] but against an explicit `DATABASE_URL`. Used by tests that
/// need a file-backed DB (e.g. the backup endpoint: through sqlx, `VACUUM INTO`
/// on an in-memory database silently produces no output file, whereas a real
/// deployment is always file-backed).
pub async fn open_app_with_url(db_url: &str) -> axum::Router {
    let db = kammerz::db::init(db_url).await.unwrap();
    let config = AppConfig {
        password_hash: None,
        ..AppConfig::default()
    };
    kammerz::routes::create_router(AppState {
        db,
        config,
        // The backup endpoint opens a separate snapshot connection to this URL
        // (kammerz-vlyu.16), so file-backed tests must carry the real path.
        db_url: db_url.to_string(),
    })
}

/// Like [`open_app`] but also returns the DB connection, for tests that must
/// seed state the API no longer allows (e.g. legacy rows that predate newer
/// invariants such as the lab/self dev mutual-exclusion guard).
pub async fn open_app_with_db() -> (axum::Router, sea_orm::DatabaseConnection) {
    let db = kammerz::db::init("sqlite::memory:").await.unwrap();
    let config = AppConfig {
        password_hash: None,
        ..AppConfig::default()
    };
    let app = kammerz::routes::create_router(AppState {
        db: db.clone(),
        config,
        // NOTE: the backup endpoint opens a SEPARATE connection to this URL
        // (kammerz-vlyu.16), and `sqlite::memory:` resolves to a fresh EMPTY DB —
        // not the served one. A `/api/backup` test must use `open_app_with_url`
        // with a file path (see tests/backup.rs), never this helper.
        db_url: "sqlite::memory:".to_string(),
    });
    (app, db)
}

/// Build an app with a single password configured, backed by a fresh in-memory
/// DB and a `tower-sessions` session layer (required because `RequireAuth`
/// consults the session when a password hash is set). `min_connections(1)` keeps
/// the in-memory session DB alive for the life of the pool.
pub async fn app_with_password(pw: &str) -> axum::Router {
    let db = kammerz::db::init("sqlite::memory:").await.unwrap();
    let config = AppConfig {
        password_hash: Some(kammerz::auth::password::hash_password(pw).unwrap()),
        ..AppConfig::default()
    };
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .min_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = tower_sessions_sqlx_store::SqliteStore::new(pool);
    store.migrate().await.unwrap();
    let layer = tower_sessions::SessionManagerLayer::new(store);
    kammerz::routes::create_router(AppState {
        db,
        config,
        // In-memory URL — not usable for a `/api/backup` test (see the note in
        // `open_app_with_db`).
        db_url: "sqlite::memory:".to_string(),
    })
    .layer(layer)
}

/// Build a GET request with an empty body.
pub fn get(path: &str) -> Request<Body> {
    Request::builder().uri(path).body(Body::empty()).unwrap()
}

/// Build a DELETE request (no body).
pub fn delete(path: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

/// Build a POST request with a JSON body.
pub fn post_json<T: Serialize>(path: &str, value: &T) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(value).unwrap()))
        .unwrap()
}

/// Build a PUT request with a JSON body.
pub fn put_json<T: Serialize>(path: &str, value: &T) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(value).unwrap()))
        .unwrap()
}

/// Deserialize a response body as JSON.
pub async fn json_body<T: DeserializeOwned>(response: Response) -> T {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
