//! Shared test helpers for integration tests.
//!
//! `mod common;` is compiled independently into each test binary, so any helper
//! a given test file doesn't use shows as dead code there. Allow it module-wide.
#![allow(dead_code)]

use axum::body::Body;
use axum::http::Request;
use axum::response::Response;
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

use kammerz::config::AppConfig;
use kammerz::AppState;

/// Build an app in OPEN auth mode (no password) backed by a fresh in-memory DB.
/// In open mode `RequireAuth` passes without a session, so no session layer is needed.
pub async fn open_app() -> axum::Router {
    let db = kammerz::db::init("sqlite::memory:").await.unwrap();
    let config = AppConfig {
        password_hash: None,
        anthropic_api_key: None,
        secure_cookies: false,
    };
    kammerz::routes::create_router(AppState { db, config })
}

/// Build a GET request with an empty body.
pub fn get(path: &str) -> Request<Body> {
    Request::builder()
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

/// Deserialize a response body as JSON.
pub async fn json_body<T: DeserializeOwned>(response: Response) -> T {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
