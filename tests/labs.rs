mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json};
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

// kammerz-o0l: deleting a missing lab returns 404 NOT_FOUND, not a no-op 204.
#[tokio::test]
async fn delete_missing_lab_returns_404() {
    let app = open_app().await;

    let res = app.oneshot(delete("/api/labs/999999")).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "Lab 999999 not found");
}
