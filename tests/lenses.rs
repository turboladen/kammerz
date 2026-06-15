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

// --- Full update + distinct/association endpoints (kammerz-do4) ---

/// Exercise every setter branch in the lens `update` handler in one PUT — the
/// existing partial-patch test only touches model + focal_length, leaving the
/// brand / mount / aperture / filter / serial / date / notes setters untested.
#[tokio::test]
async fn update_lens_sets_every_field() {
    let app = open_app().await;
    let mount_id = seeded_lens_mount_id(&app).await;
    let id = create_lens(&app, "Before").await;

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/lenses/{id}"),
            &json!({
                "brand": "Afterar",
                "lens_mount_id": mount_id,
                "lens_system": "Nikkor",
                "model": "After 55",
                "focal_length": "55mm",
                "max_aperture": "f/1.4",
                "min_aperture": "f/16",
                "filter_thread_front_mm": 52,
                "filter_thread_rear_mm": 40,
                "serial_number": "SN-12345",
                "date_purchased": "2026-01-02",
                "purchased_from": "KEH",
                "date_sold": "2026-03-04",
                "notes": "minty"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get(&format!("/api/lenses/{id}")))
        .await
        .unwrap();
    let lens: Value = json_body(res).await;
    assert_eq!(lens["brand"], "Afterar");
    assert_eq!(lens["lens_system"], "Nikkor");
    assert_eq!(lens["model"], "After 55");
    assert_eq!(lens["max_aperture"], "f/1.4");
    assert_eq!(lens["filter_thread_front_mm"], 52);
    assert_eq!(lens["filter_thread_rear_mm"], 40);
    assert_eq!(lens["serial_number"], "SN-12345");
    assert_eq!(lens["date_purchased"], "2026-01-02");
    assert_eq!(lens["purchased_from"], "KEH");
    assert_eq!(lens["date_sold"], "2026-03-04");
    assert_eq!(lens["notes"], "minty");
}

#[tokio::test]
async fn update_lens_rejects_invalid_date() {
    let app = open_app().await;
    let id = create_lens(&app, "Datear").await;
    let res = app
        .oneshot(put_json(
            &format!("/api/lenses/{id}"),
            &json!({ "date_purchased": "not-a-date" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn distinct_lens_brands_lists_created_brands() {
    let app = open_app().await;
    let mount_id = seeded_lens_mount_id(&app).await;
    for brand in ["Zeta", "Alpha"] {
        let res = app
            .clone()
            .oneshot(post_json(
                "/api/lenses",
                &json!({ "brand": brand, "lens_mount_id": mount_id, "model": "x" }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    let res = app
        .oneshot(get("/api/lenses/distinct/brands"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let brands: Vec<String> = json_body(res).await;
    assert!(brands.contains(&"Alpha".to_string()) && brands.contains(&"Zeta".to_string()));
    // distinct_brands sorts ascending — Alpha precedes Zeta.
    let (ai, zi) = (
        brands.iter().position(|b| b == "Alpha").unwrap(),
        brands.iter().position(|b| b == "Zeta").unwrap(),
    );
    assert!(ai < zi, "brands come back ASC-sorted: {brands:?}");
}

#[tokio::test]
async fn distinct_lens_systems_lists_only_non_null_systems() {
    let app = open_app().await;
    let mount_id = seeded_lens_mount_id(&app).await;
    // One lens with a system, one without — only the set system should appear.
    for body in [
        json!({ "brand": "Sys", "lens_mount_id": mount_id, "model": "a", "lens_system": "K/L" }),
        json!({ "brand": "NoSys", "lens_mount_id": mount_id, "model": "b" }),
    ] {
        app.clone()
            .oneshot(post_json("/api/lenses", &body))
            .await
            .unwrap();
    }

    let res = app
        .oneshot(get("/api/lenses/distinct/systems"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let systems: Vec<String> = json_body(res).await;
    assert_eq!(systems, vec!["K/L".to_string()]);
}

#[tokio::test]
async fn cameras_for_lens_lists_linked_camera() {
    let app = open_app().await;
    let lens_id = create_lens(&app, "Linkar").await;

    // Borrow a seeded camera and link the lens to it.
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;
    let res = app
        .clone()
        .oneshot(post_json(
            &format!("/api/cameras/{camera_id}/lenses/{lens_id}"),
            &json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get(&format!("/api/lenses/{lens_id}/cameras")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let camera_ids: Vec<i32> = json_body(res).await;
    assert_eq!(camera_ids, vec![camera_id]);
}

#[tokio::test]
async fn cameras_for_lens_is_empty_when_unlinked() {
    let app = open_app().await;
    let lens_id = create_lens(&app, "Lonelens").await;
    let res = app
        .oneshot(get(&format!("/api/lenses/{lens_id}/cameras")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let camera_ids: Vec<i32> = json_body(res).await;
    assert!(camera_ids.is_empty());
}
