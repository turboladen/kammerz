mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn list_lens_mounts_returns_seeded_mounts() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/lens-mounts")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let mounts: Vec<Value> = json_body(res).await;
    assert!(!mounts.is_empty(), "migrations seed lens mounts");
}

#[tokio::test]
async fn create_lens_mount_then_appears_in_list() {
    let app = open_app().await;

    let payload = json!({ "name": "Test Bayonet 9000" });
    let res = app
        .clone()
        .oneshot(post_json("/api/lens-mounts", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let new_id: i32 = json_body(res).await;

    let res = app.oneshot(get("/api/lens-mounts")).await.unwrap();
    let mounts: Vec<Value> = json_body(res).await;
    assert!(mounts
        .iter()
        .any(|m| m["id"].as_i64().unwrap() as i32 == new_id && m["name"] == "Test Bayonet 9000"));
}

// --- Server-side input validation (kammerz-grd) ---

#[tokio::test]
async fn create_lens_mount_rejects_whitespace_name() {
    let app = open_app().await;
    let res = app
        .oneshot(post_json("/api/lens-mounts", &json!({ "name": "   " })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("name"));
}

#[tokio::test]
async fn create_lens_mount_trims_name() {
    // The fixed-lens convention detects the mount by exact name, so an untrimmed
    // "  Fixed Lens " would silently defeat it. Verify the stored value is trimmed.
    let app = open_app().await;
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/lens-mounts",
            &json!({ "name": "  Padded Mount  " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let new_id: i32 = json_body(res).await;

    let res = app.oneshot(get("/api/lens-mounts")).await.unwrap();
    let mounts: Vec<Value> = json_body(res).await;
    let mount = mounts
        .iter()
        .find(|m| m["id"].as_i64().unwrap() as i32 == new_id)
        .unwrap();
    assert_eq!(mount["name"], "Padded Mount", "name is stored trimmed");
}
