//! Coverage for the Anthropic/Claude integration in `import_service` (kammerz-4n2).
//!
//! The service's `*_at` methods take a base URL so these tests can point the real
//! reqwest client at a local axum mock server — exercising the genuine HTTP /
//! serde / fence-stripping code path against canned responses, no live API.

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};
use kammerz::services::import_service::ImportService;
use serde_json::json;

/// Bind an ephemeral port, serve the given router, and return its base URL.
async fn spawn(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

// --- list_models ---------------------------------------------------------

#[tokio::test]
async fn list_models_maps_entries() {
    let app = Router::new().route(
        "/v1/models",
        get(|| async {
            (
                StatusCode::OK,
                r#"{"data":[{"id":"claude-x","display_name":"Claude X"},{"id":"claude-y","display_name":"Claude Y"}]}"#,
            )
        }),
    );
    let base = spawn(app).await;

    let models = ImportService::list_models_at(&base, "key").await.unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "claude-x");
    assert_eq!(models[0].display_name, "Claude X");
    assert_eq!(models[1].id, "claude-y");
}

#[tokio::test]
async fn list_models_surfaces_structured_api_error() {
    let app = Router::new().route(
        "/v1/models",
        get(|| async {
            (
                StatusCode::UNAUTHORIZED,
                r#"{"error":{"type":"authentication_error","message":"invalid x-api-key"}}"#,
            )
        }),
    );
    let base = spawn(app).await;

    let err = ImportService::list_models_at(&base, "bad")
        .await
        .unwrap_err();
    assert_eq!(err, "API error (401): invalid x-api-key");
}

#[tokio::test]
async fn list_models_falls_back_on_unstructured_error_body() {
    let app = Router::new().route(
        "/v1/models",
        get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "upstream exploded") }),
    );
    let base = spawn(app).await;

    let err = ImportService::list_models_at(&base, "key")
        .await
        .unwrap_err();
    assert_eq!(err, "Anthropic API returned HTTP 500.");
}

#[tokio::test]
async fn list_models_errors_on_malformed_success_body() {
    let app = Router::new().route(
        "/v1/models",
        get(|| async { (StatusCode::OK, r#"{"unexpected":"shape"}"#) }),
    );
    let base = spawn(app).await;

    let err = ImportService::list_models_at(&base, "key")
        .await
        .unwrap_err();
    assert!(
        err.starts_with("Failed to parse models response:"),
        "got: {err}"
    );
}

#[tokio::test]
async fn list_models_errors_when_host_unreachable() {
    // Nothing listens on port 1 → connection refused (fast, not a timeout).
    let err = ImportService::list_models_at("http://127.0.0.1:1", "key")
        .await
        .unwrap_err();
    assert!(
        err.starts_with("Failed to reach Anthropic API:"),
        "got: {err}"
    );
}

// --- parse_note ----------------------------------------------------------

/// Build a `/v1/messages`-style response whose single text block is `text`.
fn messages_response(text: &str) -> String {
    json!({ "content": [{ "type": "text", "text": text }] }).to_string()
}

#[tokio::test]
async fn parse_note_parses_clean_json() {
    let inner = r#"{"roll_id":"NFE-1","film_stock_guess":"Portra 400","frame_count":36,"shots":[{"frame_number":"1","aperture":"f/8"},{"frame_number":"2"}]}"#;
    let body = messages_response(inner);
    let app = Router::new().route(
        "/v1/messages",
        post(move || async move { (StatusCode::OK, body) }),
    );
    let base = spawn(app).await;

    let roll = ImportService::parse_note_at(&base, "key", "claude-x", "note")
        .await
        .unwrap();
    assert_eq!(roll.roll_id, "NFE-1");
    assert_eq!(roll.film_stock_guess.as_deref(), Some("Portra 400"));
    assert_eq!(roll.frame_count, Some(36));
    assert_eq!(roll.shots.len(), 2);
    assert_eq!(roll.shots[0].frame_number, "1");
    assert_eq!(roll.shots[0].aperture.as_deref(), Some("f/8"));
}

#[tokio::test]
async fn parse_note_strips_markdown_code_fences() {
    let inner = r#"{"roll_id":"M67-24","shots":[{"frame_number":"H1"}]}"#;
    let fenced = format!("```json\n{inner}\n```");
    let body = messages_response(&fenced);
    let app = Router::new().route(
        "/v1/messages",
        post(move || async move { (StatusCode::OK, body) }),
    );
    let base = spawn(app).await;

    let roll = ImportService::parse_note_at(&base, "key", "claude-x", "note")
        .await
        .unwrap();
    assert_eq!(roll.roll_id, "M67-24");
    assert_eq!(roll.shots[0].frame_number, "H1");
}

#[tokio::test]
async fn parse_note_surfaces_structured_api_error() {
    let app = Router::new().route(
        "/v1/messages",
        post(|| async {
            (
                StatusCode::BAD_REQUEST,
                r#"{"error":{"type":"invalid_request_error","message":"max_tokens too large"}}"#,
            )
        }),
    );
    let base = spawn(app).await;

    let err = ImportService::parse_note_at(&base, "key", "claude-x", "note")
        .await
        .unwrap_err();
    assert_eq!(err, "Claude API error (400): max_tokens too large");
}

#[tokio::test]
async fn parse_note_falls_back_on_unstructured_error_body() {
    let app = Router::new().route(
        "/v1/messages",
        post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "gateway boom") }),
    );
    let base = spawn(app).await;

    let err = ImportService::parse_note_at(&base, "key", "claude-x", "note")
        .await
        .unwrap_err();
    assert_eq!(
        err,
        "Claude API returned HTTP 500. Check the application logs for details."
    );
}

#[tokio::test]
async fn parse_note_errors_on_empty_content() {
    let app = Router::new().route(
        "/v1/messages",
        post(|| async { (StatusCode::OK, r#"{"content":[]}"#) }),
    );
    let base = spawn(app).await;

    let err = ImportService::parse_note_at(&base, "key", "claude-x", "note")
        .await
        .unwrap_err();
    assert_eq!(err, "Claude API returned no text content");
}

#[tokio::test]
async fn parse_note_errors_on_non_json_model_output() {
    let body = messages_response("I'm sorry, I can't help with that.");
    let app = Router::new().route(
        "/v1/messages",
        post(move || async move { (StatusCode::OK, body) }),
    );
    let base = spawn(app).await;

    let err = ImportService::parse_note_at(&base, "key", "claude-x", "note")
        .await
        .unwrap_err();
    assert!(
        err.starts_with("Failed to parse AI response as structured data:"),
        "got: {err}"
    );
}

#[tokio::test]
async fn parse_note_errors_on_malformed_messages_envelope() {
    // 200 OK but the body isn't a MessagesResponse at all.
    let app = Router::new().route(
        "/v1/messages",
        post(|| async { (StatusCode::OK, r#"{"nope":true}"#) }),
    );
    let base = spawn(app).await;

    let err = ImportService::parse_note_at(&base, "key", "claude-x", "note")
        .await
        .unwrap_err();
    assert!(
        err.starts_with("Failed to parse Claude API response:"),
        "got: {err}"
    );
}
