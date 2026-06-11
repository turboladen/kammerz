mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json, put_json};
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

// --- Update / delete (kammerz-6l5) ---

/// Create a lens on a seeded mount and return its id.
async fn create_lens(app: &axum::Router, model: &str) -> i32 {
    let res = app.clone().oneshot(get("/api/lens-mounts")).await.unwrap();
    let mounts: Vec<Value> = json_body(res).await;
    let mount_id = mounts[0]["id"].as_i64().unwrap() as i32;

    let payload = json!({
        "brand": "Testar",
        "lens_mount_id": mount_id,
        "model": model,
        "focal_length": "50mm",
        "max_aperture": "f/2"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/lenses", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

#[tokio::test]
async fn update_lens_applies_partial_patch() {
    let app = open_app().await;
    let id = create_lens(&app, "Patchar 50").await;

    // Partial update: change model, clear focal_length (double_option null).
    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/lenses/{id}"),
            &json!({ "model": "Patchar 55", "focal_length": null }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get(&format!("/api/lenses/{id}")))
        .await
        .unwrap();
    let lens: Value = json_body(res).await;
    assert_eq!(lens["model"], "Patchar 55");
    assert!(
        lens["focal_length"].is_null(),
        "explicit null clears the field via double_option"
    );
    assert_eq!(lens["brand"], "Testar", "untouched field survives the patch");
    assert_eq!(lens["max_aperture"], "f/2");
}

#[tokio::test]
async fn update_missing_lens_is_404() {
    let app = open_app().await;
    let res = app
        .oneshot(put_json("/api/lenses/999999", &json!({ "model": "ghost" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_lens_removes_it() {
    let app = open_app().await;
    let id = create_lens(&app, "Deletar 50").await;

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/lenses/{id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // get_one returns Json<Option<Model>> → 200 with a null body once gone.
    let res = app
        .oneshot(get(&format!("/api/lenses/{id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let lens: Value = json_body(res).await;
    assert!(lens.is_null(), "deleted lens reads back as null");
}
