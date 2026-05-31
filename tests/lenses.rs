mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn list_lenses_returns_200() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/lenses")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _lenses: Vec<Value> = json_body(res).await;
}

#[tokio::test]
async fn create_then_get_lens_roundtrips() {
    let app = open_app().await;

    // Borrow a valid lens_mount_id from the seeded mounts.
    let res = app.clone().oneshot(get("/api/lens-mounts")).await.unwrap();
    let mounts: Vec<Value> = json_body(res).await;
    let mount_id = mounts[0]["id"].as_i64().unwrap() as i32;

    let payload = json!({
        "brand": "Testar",
        "lens_mount_id": mount_id,
        "model": "Sonnar 50",
        "focal_length": "50mm",
        "max_aperture": "f/2"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/lenses", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let new_id: i32 = json_body(res).await;

    let res = app
        .oneshot(get(&format!("/api/lenses/{new_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let lens: Value = json_body(res).await;
    assert_eq!(lens["id"].as_i64().unwrap() as i32, new_id);
    assert_eq!(lens["brand"], "Testar");
}
