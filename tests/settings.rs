mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, put_json};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn set_then_get_setting_roundtrips() {
    let app = open_app().await;

    // Unknown key initially returns null.
    let res = app
        .clone()
        .oneshot(get("/api/settings/test_key"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val: Value = json_body(res).await;
    assert!(val.is_null());

    // PUT a value → 204.
    let res = app
        .clone()
        .oneshot(put_json(
            "/api/settings/test_key",
            &serde_json::json!({ "value": "hello" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // GET returns the value.
    let res = app.oneshot(get("/api/settings/test_key")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let val: Value = json_body(res).await;
    assert_eq!(val, "hello");
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
