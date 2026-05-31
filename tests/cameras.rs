mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn list_cameras_returns_seeded_gear() {
    // Fresh in-memory DB runs all migrations incl. the user's seeded cameras.
    let app = open_app().await;
    let res = app.oneshot(get("/api/cameras")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cams: Vec<Value> = json_body(res).await;
    assert!(!cams.is_empty(), "migrations seed the user's cameras");
}

#[tokio::test]
async fn create_then_get_camera_roundtrips() {
    let app = open_app().await;

    // Borrow a valid lens_mount_id from a seeded camera.
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let mount_id = cams[0]["lens_mount_id"].as_i64().unwrap() as i32;

    // POST a new camera → 201 with the new id.
    let payload = json!({
        "brand": "Testarossa",
        "model": "T-1000",
        "format": "35mm",
        "lens_mount_id": mount_id,
        "camera_type": "SLR"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/cameras", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let new_id: i32 = json_body(res).await;

    // GET /api/cameras/{id} → returns that camera.
    let res = app
        .oneshot(get(&format!("/api/cameras/{new_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cam: Value = json_body(res).await;
    assert_eq!(cam["id"].as_i64().unwrap() as i32, new_id);
    assert_eq!(cam["brand"], "Testarossa");
    assert_eq!(cam["model"], "T-1000");
}
