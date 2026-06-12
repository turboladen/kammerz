mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json, put_json};
use serde_json::{Value, json};
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

// --- Server-side input validation (kammerz-grd) ---

#[tokio::test]
async fn create_lab_rejects_whitespace_name() {
    let app = open_app().await;
    let res = app
        .oneshot(post_json("/api/labs", &json!({ "name": "   " })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("name"));
}

#[tokio::test]
async fn create_lab_trims_name() {
    let app = open_app().await;
    let res = app
        .clone()
        .oneshot(post_json("/api/labs", &json!({ "name": "  Padded Lab  " })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let id: i32 = json_body(res).await;
    let res = app.oneshot(get(&format!("/api/labs/{id}"))).await.unwrap();
    let lab: Value = json_body(res).await;
    assert_eq!(lab["name"], "Padded Lab", "name is stored trimmed");
}

#[tokio::test]
async fn update_lab_rejects_whitespace_name() {
    let app = open_app().await;
    let res = app
        .clone()
        .oneshot(post_json("/api/labs", &json!({ "name": "Keepme" })))
        .await
        .unwrap();
    let id: i32 = json_body(res).await;
    let res = app
        .oneshot(put_json(
            &format!("/api/labs/{id}"),
            &json!({ "name": " " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
