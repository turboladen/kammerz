mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn list_labs_returns_200() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/labs")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _labs: Vec<Value> = json_body(res).await;
}

#[tokio::test]
async fn create_then_get_lab_roundtrips() {
    let app = open_app().await;

    let payload = json!({
        "name": "Test Lab",
        "location": "Portland, OR",
        "website": "https://example.com"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/labs", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let new_id: i32 = json_body(res).await;

    let res = app
        .oneshot(get(&format!("/api/labs/{new_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let lab: Value = json_body(res).await;
    assert_eq!(lab["id"].as_i64().unwrap() as i32, new_id);
    assert_eq!(lab["name"], "Test Lab");
}
