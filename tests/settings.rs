mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, put_json};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn set_then_get_setting_roundtrips() {
    let app = open_app().await;

    // Use a known, non-secret key (`claude_model`): the allowlist rejects unknown
    // keys (kammerz-vlyu.17), and a secret key would mask its value on GET.
    // A not-yet-set known key returns null.
    let res = app
        .clone()
        .oneshot(get("/api/settings/claude_model"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val: Value = json_body(res).await;
    assert!(val.is_null());

    // PUT a value → 204.
    let res = app
        .clone()
        .oneshot(put_json(
            "/api/settings/claude_model",
            &serde_json::json!({ "value": "claude-sonnet-4-5" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // GET returns the value.
    let res = app
        .oneshot(get("/api/settings/claude_model"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val: Value = json_body(res).await;
    assert_eq!(val, "claude-sonnet-4-5");
}

#[tokio::test]
async fn unknown_setting_key_is_rejected() {
    let app = open_app().await;

    // Writing an unrecognized key must not silently store it (kammerz-vlyu.17).
    let res = app
        .oneshot(put_json(
            "/api/settings/bogus_key",
            &serde_json::json!({ "value": "whatever" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let val: Value = json_body(res).await;
    assert_eq!(val["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn setting_value_is_trimmed_on_write() {
    let app = open_app().await;

    // Surrounding whitespace is stripped before storage.
    let res = app
        .clone()
        .oneshot(put_json(
            "/api/settings/claude_model",
            &serde_json::json!({ "value": "  claude-opus-4  " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get("/api/settings/claude_model"))
        .await
        .unwrap();
    let val: Value = json_body(res).await;
    assert_eq!(val, "claude-opus-4");
}

#[tokio::test]
async fn whitespace_only_api_key_reads_as_unset() {
    let app = open_app().await;

    // A whitespace-only secret trims to empty → stored empty → reads back as
    // "not configured" (null, not the mask), matching an unset key (kammerz-vlyu.17).
    let res = app
        .clone()
        .oneshot(put_json(
            "/api/settings/claude_api_key",
            &serde_json::json!({ "value": "   " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get("/api/settings/claude_api_key"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val: Value = json_body(res).await;
    assert!(
        val.is_null(),
        "whitespace-only API key should read as unset"
    );
}

#[tokio::test]
async fn claude_api_key_is_write_only() {
    let app = open_app().await;

    // No key saved → null (so the UI can tell "not configured").
    let res = app
        .clone()
        .oneshot(get("/api/settings/claude_api_key"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val: Value = json_body(res).await;
    assert!(val.is_null());

    // PUT still accepts the secret normally.
    let res = app
        .clone()
        .oneshot(put_json(
            "/api/settings/claude_api_key",
            &serde_json::json!({ "value": "sk-ant-super-secret" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // GET never returns the cleartext — only the masked sentinel.
    let res = app
        .clone()
        .oneshot(get("/api/settings/claude_api_key"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val: Value = json_body(res).await;
    assert_eq!(val, "********");

    // An empty saved value still reads as "not configured" (null, not mask).
    let res = app
        .clone()
        .oneshot(put_json(
            "/api/settings/claude_api_key",
            &serde_json::json!({ "value": "" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let res = app
        .oneshot(get("/api/settings/claude_api_key"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val: Value = json_body(res).await;
    assert!(val.is_null());
}
