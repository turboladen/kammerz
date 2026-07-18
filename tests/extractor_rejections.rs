//! axum's built-in extractor rejections (malformed JSON, wrong types, invalid
//! enums, non-numeric path ids, missing query params) bypass `AppError` and
//! return plain-text bodies by default. The wrapper extractors in
//! `src/extract.rs` convert each rejection into the standard
//! `{"error":{"code","message"}}` envelope so the frontend client never falls
//! back to opaque `statusText` + `UNKNOWN` (kammerz-4lr). These tests pin the
//! envelope SHAPE (both `error.code` and `error.message` present) and the status
//! axum assigns to each rejection class.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{get, json_body, open_app};
use serde_json::Value;
use tower::ServiceExt;

/// Assert the response carries the standard error envelope: a `200`-style JSON
/// body with both `error.code` and `error.message` present and non-empty.
async fn assert_envelope(res: axum::response::Response) -> (String, String) {
    let body: Value = json_body(res).await;
    let code = body["error"]["code"]
        .as_str()
        .expect("error.code must be a present string")
        .to_string();
    let message = body["error"]["message"]
        .as_str()
        .expect("error.message must be a present string")
        .to_string();
    assert!(!code.is_empty(), "error.code must not be empty");
    assert!(!message.is_empty(), "error.message must not be empty");
    (code, message)
}

/// POST with a raw (unserialized) body so we can send deliberately malformed
/// JSON. `post_json` would force-serialize a value, which can't be malformed.
fn post_raw(path: &str, content_type: Option<&str>, body: &str) -> Request<Body> {
    let mut builder = Request::builder().method("POST").uri(path);
    if let Some(ct) = content_type {
        builder = builder.header("content-type", ct);
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn malformed_json_body_is_400_envelope() {
    let app = open_app().await;
    // Truncated object — a JSON syntax error, which axum maps to 400.
    let res = app
        .oneshot(post_raw(
            "/api/rolls",
            Some("application/json"),
            "{ \"roll_id\": ",
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let (_code, message) = assert_envelope(res).await;
    // axum's body_text() preserves serde's positional diagnostic.
    assert!(
        message.to_lowercase().contains("line") || message.to_lowercase().contains("column"),
        "expected serde positional text in message, got: {message}"
    );
}

#[tokio::test]
async fn invalid_enum_value_is_422_envelope() {
    let app = open_app().await;
    // Well-formed JSON, but `push_pull` is not a valid PushPull variant. axum
    // classifies a deserialize (data) error as 422. (`status` was retired with the
    // enum — ADR-0013 — so push_pull is now the roll's constrained-enum field.)
    let res = app
        .oneshot(post_raw(
            "/api/rolls",
            Some("application/json"),
            r#"{"roll_id":"A1","push_pull":"bogus"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let (_code, message) = assert_envelope(res).await;
    // The message should name the offending field so enum drift (Rust vs TS) is
    // diagnosable from the client.
    assert!(
        message.contains("push_pull"),
        "expected the field name `push_pull` in message, got: {message}"
    );
}

#[tokio::test]
async fn missing_content_type_is_415_envelope() {
    let app = open_app().await;
    // Valid JSON bytes but no `Content-Type: application/json` header → axum's
    // Json extractor rejects with 415 before attempting to deserialize.
    let res = app
        .oneshot(post_raw("/api/rolls", None, r#"{"roll_id":"A1"}"#))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_envelope(res).await;
}

#[tokio::test]
async fn non_numeric_path_id_is_400_envelope() {
    let app = open_app().await;
    // `abc` can't parse into the `i32` path param → axum Path rejection (400).
    let res = app.oneshot(get("/api/rolls/abc")).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_envelope(res).await;
}

#[tokio::test]
async fn missing_required_query_param_is_400_envelope() {
    let app = open_app().await;
    // `SearchQuery { q }` has no default → omitting `?q=` is a Query rejection.
    let res = app.oneshot(get("/api/search")).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert_envelope(res).await;
}
