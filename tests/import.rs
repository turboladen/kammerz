use axum::http::StatusCode;
use tower::ServiceExt;

mod common;
use common::{get, json_body, open_app};

#[tokio::test]
async fn list_models_without_key_is_422() {
    // open_app() configures no password and no anthropic_api_key, and the fresh
    // in-memory DB has no `claude_api_key` settings row — so key resolution fails
    // before any network call to Anthropic.
    let app = open_app().await;
    let res = app.oneshot(get("/api/import/models")).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let v: serde_json::Value = json_body(res).await;
    let msg = v["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("No Anthropic API key"),
        "expected the no-API-key message, got: {msg}"
    );
}
