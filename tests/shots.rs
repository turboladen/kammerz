mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json, put_json};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn first_camera_id(app: &axum::Router) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    cams[0]["id"].as_i64().unwrap() as i32
}

async fn create_loaded_roll(app: &axum::Router, roll_id: &str) -> i32 {
    create_roll_at_status(app, roll_id, "loaded").await
}

/// Create a roll directly at a given status (mirrors tests/development.rs).
async fn create_roll_at_status(app: &axum::Router, roll_id: &str, status: &str) -> i32 {
    let camera_id = first_camera_id(app).await;
    let payload = json!({
        "roll_id": roll_id,
        "camera_id": camera_id,
        "status": status,
        "date_loaded": "2026-05-01"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

#[tokio::test]
async fn create_shot_transaction_links_lens_and_syncs_status() {
    let app = open_app().await;
    let roll_pk = create_loaded_roll(&app, "SHOT-TXN").await;

    // Borrow a valid lens id from seeded data.
    let res = app.clone().oneshot(get("/api/lenses")).await.unwrap();
    let lenses: Vec<Value> = json_body(res).await;
    assert!(!lenses.is_empty(), "migrations seed lenses");
    let lens_id = lenses[0]["id"].as_i64().unwrap() as i32;

    // POST a shot with a lens linkage — exercises the transactional create.
    let payload = json!({
        "roll_id": roll_pk,
        "frame_number": "1",
        "aperture": "f/8",
        "lens_ids": [lens_id]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/shots", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let shot_id: i32 = json_body(res).await;

    // list_for_roll shows the new shot.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/shots/for-roll/{roll_pk}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let shots: Vec<Value> = json_body(res).await;
    assert_eq!(shots.len(), 1);
    assert_eq!(shots[0]["frame_number"], "1");

    // The lens linkage persisted inside the transaction.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/shots/{shot_id}/lenses")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let linked: Vec<i32> = json_body(res).await;
    assert_eq!(linked, vec![lens_id], "lens linked via set_lenses_for_shot");

    // Count increments.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/shots/for-roll/{roll_pk}/count")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let count: u64 = json_body(res).await;
    assert_eq!(count, 1);

    // Roll status auto-synced loaded → shooting inside the same transaction.
    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(
        roll["status"], "shooting",
        "auto_sync_status advanced the roll"
    );
}

#[tokio::test]
async fn delete_last_shot_reverts_status() {
    let app = open_app().await;
    let roll_pk = create_loaded_roll(&app, "SHOT-DEL").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/shots",
            &json!({ "roll_id": roll_pk, "frame_number": "1" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let shot_id: i32 = json_body(res).await;

    // Deleting the only shot should revert shooting → loaded.
    let res = app
        .clone()
        .oneshot(delete(&format!("/api/shots/{shot_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(
        roll["status"], "loaded",
        "auto_sync_status reverted the roll"
    );
}

// kammerz-8rh no-regression: the delete-last-shot revert is scoped to
// shooting/shot → loaded. A roll already past 'shot' (e.g. at-lab) that has its
// only shot deleted must NOT be pulled back to loaded — the dev pipeline status
// outranks shot bookkeeping.
#[tokio::test]
async fn delete_last_shot_past_shot_leaves_status_untouched() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "SHOT-DEL-ATLAB", "at-lab").await;

    // Adding a shot is sync'd only for loaded → shooting; at-lab is unchanged.
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/shots",
            &json!({ "roll_id": roll_pk, "frame_number": "1" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let shot_id: i32 = json_body(res).await;

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/shots/{shot_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(
        roll["status"], "at-lab",
        "deleting the last shot of an at-lab roll must not revert it to loaded"
    );
}

// kammerz-rwa: deleting a shot that doesn't exist (e.g. a stale-id double-delete
// from the frontend) must return 404 NOT_FOUND, not 422. The lookup runs inside
// the txn closure; or_404_db + friendly_txn_err classify the resulting
// DbErr::RecordNotFound as a 404, matching non-transactional handlers.
#[tokio::test]
async fn delete_missing_shot_returns_404() {
    let app = open_app().await;

    let res = app.oneshot(delete("/api/shots/999999")).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "Shot 999999 not found");
}

// kammerz-956: creating a shot against a roll that doesn't exist (e.g. a stale
// roll id after the roll was deleted on another device) trips the shots.roll_id
// FK on INSERT. The 422 message must describe the missing reference — not claim
// the user tried to delete the shot, which is the operation they didn't perform.
#[tokio::test]
async fn create_shot_for_missing_roll_is_friendly_422_not_delete_wording() {
    let app = open_app().await;

    let payload = json!({
        "roll_id": 999999,
        "frame_number": "1",
    });
    let res = app
        .oneshot(post_json("/api/shots", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        !msg.contains("FOREIGN KEY"),
        "friendly_err should rewrite the raw constraint error, got: {msg}"
    );
    assert!(
        !msg.to_lowercase().contains("delete"),
        "create-path FK violation must not use delete wording, got: {msg}"
    );
    assert!(
        msg.contains("no longer exists"),
        "create-path FK violation should say the referenced record is missing, got: {msg}"
    );
}

// --- Server-side input validation (kammerz-grd) ---

/// Create a valid shot on a fresh loaded roll, returning (shot_id, roll_pk).
async fn create_shot(app: &axum::Router, roll_id: &str) -> (i32, i32) {
    let roll_pk = create_loaded_roll(app, roll_id).await;
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/shots",
            &json!({ "roll_id": roll_pk, "frame_number": "1" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let shot_id: i32 = json_body(res).await;
    (shot_id, roll_pk)
}

#[tokio::test]
async fn create_shot_rejects_whitespace_frame_number() {
    let app = open_app().await;
    let roll_pk = create_loaded_roll(&app, "SHOT-BLANK-FRAME").await;
    let res = app
        .oneshot(post_json(
            "/api/shots",
            &json!({ "roll_id": roll_pk, "frame_number": "   " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("frame_number"));
}

#[tokio::test]
async fn create_shot_rejects_out_of_range_latitude() {
    let app = open_app().await;
    let roll_pk = create_loaded_roll(&app, "SHOT-BAD-LAT").await;
    let res = app
        .oneshot(post_json(
            "/api/shots",
            &json!({ "roll_id": roll_pk, "frame_number": "1", "gps_lat": 91.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("gps_lat"));
}

#[tokio::test]
async fn create_shot_accepts_boundary_coordinates() {
    let app = open_app().await;
    let roll_pk = create_loaded_roll(&app, "SHOT-EDGE-GPS").await;
    let res = app
        .oneshot(post_json(
            "/api/shots",
            &json!({
                "roll_id": roll_pk,
                "frame_number": "1",
                "gps_lat": -90.0,
                "gps_lon": 180.0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn update_shot_rejects_out_of_range_longitude() {
    let app = open_app().await;
    let (shot_id, _roll_pk) = create_shot(&app, "SHOT-UPD-LON").await;
    let res = app
        .oneshot(put_json(
            &format!("/api/shots/{shot_id}"),
            &json!({ "gps_lon": -181.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn update_shot_rejects_whitespace_frame_number() {
    let app = open_app().await;
    let (shot_id, _roll_pk) = create_shot(&app, "SHOT-UPD-FRAME").await;
    let res = app
        .oneshot(put_json(
            &format!("/api/shots/{shot_id}"),
            &json!({ "frame_number": "  " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
