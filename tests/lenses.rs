mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json, put_json};
use serde_json::{Value, json};
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
    assert_eq!(
        lens["brand"], "Testar",
        "untouched field survives the patch"
    );
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

// kammerz-o0l: deleting a missing lens returns 404 NOT_FOUND, not a no-op 204.
#[tokio::test]
async fn delete_missing_lens_returns_404() {
    let app = open_app().await;

    let res = app.oneshot(delete("/api/lenses/999999")).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "Lens 999999 not found");
}

// --- Server-side input validation (kammerz-grd) ---

/// Borrow a seeded lens_mount_id.
async fn seeded_lens_mount_id(app: &axum::Router) -> i32 {
    let res = app.clone().oneshot(get("/api/lens-mounts")).await.unwrap();
    let mounts: Vec<Value> = json_body(res).await;
    mounts[0]["id"].as_i64().unwrap() as i32
}

#[tokio::test]
async fn create_lens_rejects_whitespace_brand() {
    let app = open_app().await;
    let mount_id = seeded_lens_mount_id(&app).await;
    let res = app
        .oneshot(post_json(
            "/api/lenses",
            &json!({ "brand": "  ", "lens_mount_id": mount_id, "model": "Nope" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("brand"));
}

#[tokio::test]
async fn create_lens_rejects_negative_filter_thread() {
    let app = open_app().await;
    let mount_id = seeded_lens_mount_id(&app).await;
    let res = app
        .oneshot(post_json(
            "/api/lenses",
            &json!({
                "brand": "Threadly",
                "lens_mount_id": mount_id,
                "filter_thread_front_mm": -52
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("filter_thread_front_mm")
    );
}

#[tokio::test]
async fn update_lens_rejects_negative_filter_thread() {
    let app = open_app().await;
    let lens_id = create_lens(&app, "Patchlens").await;
    let res = app
        .oneshot(put_json(
            &format!("/api/lenses/{lens_id}"),
            &json!({ "filter_thread_rear_mm": -1 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
