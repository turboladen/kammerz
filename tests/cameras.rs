mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json, put_json};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Borrow a valid lens_mount_id from a seeded camera.
async fn seeded_mount_id(app: &axum::Router) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    cams[0]["lens_mount_id"].as_i64().unwrap() as i32
}

/// Create a plain camera and return its id.
async fn create_camera(app: &axum::Router, brand: &str, model: &str) -> i32 {
    let mount_id = seeded_mount_id(app).await;
    let payload = json!({
        "brand": brand,
        "model": model,
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
    json_body(res).await
}

/// Create a lens on the given mount and return its id.
async fn create_lens(app: &axum::Router, mount_id: i32) -> i32 {
    let payload = json!({
        "brand": "Testar",
        "lens_mount_id": mount_id,
        "model": "Linkon 85",
        "focal_length": "85mm",
        "max_aperture": "f/1.8"
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

// --- Update / delete (kammerz-6l5) ---

#[tokio::test]
async fn update_camera_applies_partial_patch() {
    let app = open_app().await;
    let id = create_camera(&app, "Patchley", "P-1").await;

    // Partial update: change the model and set notes; brand must survive.
    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/cameras/{id}"),
            &json!({ "model": "P-2", "notes": "updated" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get(&format!("/api/cameras/{id}")))
        .await
        .unwrap();
    let cam: Value = json_body(res).await;
    assert_eq!(
        cam["brand"], "Patchley",
        "untouched field survives the patch"
    );
    assert_eq!(cam["model"], "P-2");
    assert_eq!(cam["notes"], "updated");
}

#[tokio::test]
async fn update_missing_camera_is_404() {
    let app = open_app().await;
    let res = app
        .oneshot(put_json(
            "/api/cameras/999999",
            &json!({ "model": "ghost" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_camera_removes_it() {
    let app = open_app().await;
    let id = create_camera(&app, "Deleton", "D-1").await;

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/cameras/{id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // get_one returns Json<Option<Model>> → 200 with a null body once gone.
    let res = app
        .oneshot(get(&format!("/api/cameras/{id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cam: Value = json_body(res).await;
    assert!(cam.is_null(), "deleted camera reads back as null");
}

// kammerz-o0l: deleting a missing camera (e.g. a stale tab / double-tap) returns
// 404 NOT_FOUND, matching the transactional delete handlers rather than the old
// 204 from a no-op delete_by_id.
#[tokio::test]
async fn delete_missing_camera_returns_404() {
    let app = open_app().await;

    let res = app.oneshot(delete("/api/cameras/999999")).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "Camera 999999 not found");
}

// --- Fixed-lens create (4-step transaction, kammerz-6l5) ---

#[tokio::test]
async fn create_with_lens_persists_camera_lens_junction_and_default() {
    let app = open_app().await;

    // The fixed-lens path uses the seeded "Fixed Lens" mount (migration 010).
    let res = app.clone().oneshot(get("/api/lens-mounts")).await.unwrap();
    let mounts: Vec<Value> = json_body(res).await;
    let fixed_mount_id = mounts
        .iter()
        .find(|m| m["name"] == "Fixed Lens")
        .expect("seeded 'Fixed Lens' mount exists")["id"]
        .as_i64()
        .unwrap() as i32;

    let payload = json!({
        "camera": {
            "brand": "Olympia",
            "model": "XB",
            "format": "35mm",
            "lens_mount_id": fixed_mount_id,
            "camera_type": "rangefinder",
            "serial_number": "SN-777"
        },
        "lens_model": "Zuikon 35mm f/2.8",
        "lens_focal_length": "35mm",
        "lens_max_aperture": "f/2.8"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/cameras/with-lens", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let camera_id: i32 = json_body(res).await;

    // 1. Camera persisted with default_lens_id pointing at the new lens.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/cameras/{camera_id}")))
        .await
        .unwrap();
    let cam: Value = json_body(res).await;
    assert_eq!(cam["brand"], "Olympia");
    assert_eq!(
        cam["lens_mount_id"].as_i64().unwrap() as i32,
        fixed_mount_id
    );
    let default_lens_id = cam["default_lens_id"]
        .as_i64()
        .expect("default_lens_id set by step 4 of the transaction")
        as i32;

    // 2. Junction row persisted: the camera's linked lenses include the new lens.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/cameras/{camera_id}/lenses")))
        .await
        .unwrap();
    let linked: Vec<i32> = json_body(res).await;
    assert_eq!(
        linked,
        vec![default_lens_id],
        "junction links exactly the built-in lens"
    );

    // 3. Lens persisted sharing brand / mount / serial with the camera.
    let res = app
        .oneshot(get(&format!("/api/lenses/{default_lens_id}")))
        .await
        .unwrap();
    let lens: Value = json_body(res).await;
    assert_eq!(lens["brand"], "Olympia");
    assert_eq!(lens["model"], "Zuikon 35mm f/2.8");
    assert_eq!(lens["focal_length"], "35mm");
    assert_eq!(lens["max_aperture"], "f/2.8");
    assert_eq!(
        lens["lens_mount_id"].as_i64().unwrap() as i32,
        fixed_mount_id
    );
    assert_eq!(lens["serial_number"], "SN-777");
}

#[tokio::test]
async fn create_with_lens_rejects_malformed_date() {
    let app = open_app().await;
    let mount_id = seeded_mount_id(&app).await;
    let payload = json!({
        "camera": {
            "brand": "Badden",
            "model": "B-1",
            "format": "35mm",
            "lens_mount_id": mount_id,
            "date_purchased": "2026-13-45"
        },
        "lens_model": "Nope 50"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/cameras/with-lens", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Validation fails before the transaction — nothing persisted.
    let res = app.oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    assert!(
        !cams.iter().any(|c| c["brand"] == "Badden"),
        "rejected camera must not be persisted"
    );
}

// --- Link / unlink lens (kammerz-6l5) ---

#[tokio::test]
async fn link_then_unlink_lens_roundtrips() {
    let app = open_app().await;
    let camera_id = create_camera(&app, "Linker", "L-1").await;
    let mount_id = seeded_mount_id(&app).await;
    let lens_id = create_lens(&app, mount_id).await;

    // Link → 204 and the lens shows up on the camera.
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
        .clone()
        .oneshot(get(&format!("/api/cameras/{camera_id}/lenses")))
        .await
        .unwrap();
    let linked: Vec<i32> = json_body(res).await;
    assert_eq!(linked, vec![lens_id]);

    // Unlink → 204 and the association is gone (lens itself survives).
    let res = app
        .clone()
        .oneshot(delete(&format!(
            "/api/cameras/{camera_id}/lenses/{lens_id}"
        )))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .clone()
        .oneshot(get(&format!("/api/cameras/{camera_id}/lenses")))
        .await
        .unwrap();
    let linked: Vec<i32> = json_body(res).await;
    assert!(linked.is_empty(), "unlink removes the junction row");

    let res = app
        .oneshot(get(&format!("/api/lenses/{lens_id}")))
        .await
        .unwrap();
    let lens: Value = json_body(res).await;
    assert!(!lens.is_null(), "unlink must not delete the lens itself");
}

#[tokio::test]
async fn link_lens_to_missing_camera_is_friendly_422() {
    let app = open_app().await;
    let mount_id = seeded_mount_id(&app).await;
    let lens_id = create_lens(&app, mount_id).await;

    let res = app
        .oneshot(post_json(
            &format!("/api/cameras/999999/lenses/{lens_id}"),
            &json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        !msg.contains("FOREIGN KEY"),
        "friendly_err should rewrite the raw constraint error, got: {msg}"
    );
}

// --- Maintenance CRUD (kammerz-6l5) ---

#[tokio::test]
async fn maintenance_crud_roundtrips() {
    let app = open_app().await;
    let camera_id = create_camera(&app, "Maintly", "M-1").await;

    // Create → 201 with the new id.
    let payload = json!({
        "camera_id": camera_id,
        "maintenance_type": "CLA",
        "done_by": "Shop A",
        "date_done": "2026-05-01",
        "cost": 120.5,
        "notes": "full service"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/maintenance", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let maint_id: i32 = json_body(res).await;

    // List for the camera includes it.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/cameras/{camera_id}/maintenance")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let records: Vec<Value> = json_body(res).await;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["id"].as_i64().unwrap() as i32, maint_id);
    assert_eq!(records[0]["maintenance_type"], "CLA");
    assert_eq!(records[0]["cost"].as_f64(), Some(120.5));

    // Partial update → 204; untouched fields survive.
    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/maintenance/{maint_id}"),
            &json!({ "maintenance_type": "repair", "notes": "shutter fixed" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .clone()
        .oneshot(get(&format!("/api/cameras/{camera_id}/maintenance")))
        .await
        .unwrap();
    let records: Vec<Value> = json_body(res).await;
    assert_eq!(records[0]["maintenance_type"], "repair");
    assert_eq!(records[0]["notes"], "shutter fixed");
    assert_eq!(records[0]["done_by"], "Shop A", "untouched field survives");

    // Delete → 204 and the list is empty again.
    let res = app
        .clone()
        .oneshot(delete(&format!("/api/maintenance/{maint_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get(&format!("/api/cameras/{camera_id}/maintenance")))
        .await
        .unwrap();
    let records: Vec<Value> = json_body(res).await;
    assert!(records.is_empty());
}

#[tokio::test]
async fn update_missing_maintenance_is_404() {
    let app = open_app().await;
    let res = app
        .oneshot(put_json(
            "/api/maintenance/999999",
            &json!({ "notes": "ghost" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// kammerz-o0l: deleting a missing maintenance record returns 404 NOT_FOUND.
#[tokio::test]
async fn delete_missing_maintenance_returns_404() {
    let app = open_app().await;

    let res = app
        .oneshot(delete("/api/maintenance/999999"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(
        body["error"]["message"],
        "Maintenance record 999999 not found"
    );
}

#[tokio::test]
async fn create_maintenance_with_malformed_date_is_rejected() {
    let app = open_app().await;
    let camera_id = create_camera(&app, "Maintly", "M-2").await;
    let payload = json!({
        "camera_id": camera_id,
        "maintenance_type": "cleaning",
        "date_done": "2026-02-30"
    });
    let res = app
        .oneshot(post_json("/api/maintenance", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}
