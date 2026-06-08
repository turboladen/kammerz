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
