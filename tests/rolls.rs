mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json, put_json};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Create a roll on a seeded camera and return its primary-key id.
async fn create_roll(app: &axum::Router, roll_id: &str) -> i32 {
    // Borrow a valid camera id from seeded data.
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let payload = json!({
        "roll_id": roll_id,
        "camera_id": camera_id,
        "status": "loaded",
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
async fn list_rolls_returns_200() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/rolls")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _rolls: Vec<Value> = json_body(res).await;
}

#[tokio::test]
async fn create_then_get_roll_roundtrips() {
    let app = open_app().await;
    let id = create_roll(&app, "TEST-001").await;

    let res = app.oneshot(get(&format!("/api/rolls/{id}"))).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let roll: Value = json_body(res).await;
    // get_with_details returns a RollWithDetails; roll_id is the user-facing label.
    assert_eq!(roll["roll_id"], "TEST-001");
}

#[tokio::test]
async fn roll_detail_composite_includes_shots() {
    let app = open_app().await;
    let roll_pk = create_roll(&app, "TEST-DETAIL").await;

    // Add a shot so the composite shots array is non-empty.
    let shot_payload = json!({
        "roll_id": roll_pk,
        "frame_number": "1"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/shots", &shot_payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    // GET the composite detail.
    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let detail: Value = json_body(res).await;

    // The composite carries the roll plus child collections.
    assert_eq!(detail["roll"]["roll_id"], "TEST-DETAIL");
    let shots = detail["shots"].as_array().expect("shots array present");
    assert_eq!(shots.len(), 1, "the one shot we added is present");
    assert_eq!(shots[0]["frame_number"], "1");
    assert!(detail["lab_dev"].is_null());
    assert!(detail["self_dev"].is_null());
    assert!(detail["dev_stages"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn roll_with_details_reports_shot_count() {
    let app = open_app().await;
    let roll_pk = create_roll(&app, "TEST-SHOTCOUNT").await;

    // A fresh roll has no shots → shot_count is 0 (not null/absent).
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(
        roll["shot_count"].as_i64(),
        Some(0),
        "a new roll reports shot_count = 0"
    );

    // Add two shots.
    for frame in ["1", "2"] {
        let res = app
            .clone()
            .oneshot(post_json(
                "/api/shots",
                &json!({ "roll_id": roll_pk, "frame_number": frame }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // shot_count increments on the single-roll endpoint…
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(
        roll["shot_count"].as_i64(),
        Some(2),
        "shot_count reflects the two shots we added"
    );

    // …and on the list endpoint that the dashboard/list frame counters use.
    let res = app.oneshot(get("/api/rolls")).await.unwrap();
    let rolls: Vec<Value> = json_body(res).await;
    let ours = rolls
        .iter()
        .find(|r| r["roll_id"] == "TEST-SHOTCOUNT")
        .expect("our roll is present in the list");
    assert_eq!(
        ours["shot_count"].as_i64(),
        Some(2),
        "the list endpoint also reports shot_count"
    );
}

// --- Date validation (kammerz-igc) ---

async fn seeded_camera_id(app: &axum::Router) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    cams[0]["id"].as_i64().unwrap() as i32
}

#[tokio::test]
async fn create_roll_with_malformed_date_is_rejected() {
    let app = open_app().await;
    let camera_id = seeded_camera_id(&app).await;
    let payload = json!({
        "roll_id": "BAD-DATE",
        "camera_id": camera_id,
        "status": "loaded",
        "date_loaded": "2026-13-45"
    });
    let res = app
        .oneshot(post_json("/api/rolls", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("date_loaded"),
        "message should name the offending field"
    );
}

#[tokio::test]
async fn create_roll_with_partial_date_is_accepted() {
    let app = open_app().await;
    let camera_id = seeded_camera_id(&app).await;
    // YYYY and YYYY-MM remain valid (matches DateInput behavior).
    let payload = json!({
        "roll_id": "PARTIAL-DATE",
        "camera_id": camera_id,
        "status": "loaded",
        "date_loaded": "2026-05"
    });
    let res = app
        .oneshot(post_json("/api/rolls", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn update_roll_with_malformed_date_is_rejected() {
    let app = open_app().await;
    let id = create_roll(&app, "UPD-BAD-DATE").await;
    let payload = json!({ "date_finished": "2026-02-30" });
    let res = app
        .oneshot(put_json(&format!("/api/rolls/{id}"), &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}

// --- Update happy path + delete cascade (kammerz-6l5) ---

#[tokio::test]
async fn update_roll_applies_partial_patch() {
    let app = open_app().await;
    let id = create_roll(&app, "UPD-OK").await;

    // Partial update: advance status, set finish date and notes;
    // camera_id / date_loaded must survive untouched.
    let payload = json!({
        "status": "shot",
        "date_finished": "2026-05-15",
        "notes": "windy day"
    });
    let res = app
        .clone()
        .oneshot(put_json(&format!("/api/rolls/{id}"), &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app.oneshot(get(&format!("/api/rolls/{id}"))).await.unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(roll["status"], "shot");
    assert_eq!(roll["date_finished"], "2026-05-15");
    assert_eq!(roll["notes"], "windy day");
    assert_eq!(
        roll["date_loaded"], "2026-05-01",
        "untouched field survives"
    );
    assert!(
        roll["camera_id"].as_i64().is_some(),
        "camera association survives the patch"
    );
}

#[tokio::test]
async fn update_missing_roll_is_404() {
    let app = open_app().await;
    let res = app
        .oneshot(put_json("/api/rolls/999999", &json!({ "notes": "ghost" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_roll_cascades_shots() {
    let app = open_app().await;
    let roll_pk = create_roll(&app, "DEL-CASCADE").await;

    // Add two shots so the FK cascade has something to clean up.
    for frame in ["1", "2"] {
        let res = app
            .clone()
            .oneshot(post_json(
                "/api/shots",
                &json!({ "roll_id": roll_pk, "frame_number": frame }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // Delete the roll → 204.
    let res = app
        .clone()
        .oneshot(delete(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // The roll is gone (get_one returns 200 + null body).
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let roll: Value = json_body(res).await;
    assert!(roll.is_null(), "deleted roll reads back as null");

    // …and its shots were cascade-deleted (shots.roll_id ON DELETE CASCADE).
    let res = app
        .oneshot(get(&format!("/api/shots/for-roll/{roll_pk}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let shots: Vec<Value> = json_body(res).await;
    assert!(shots.is_empty(), "shots are cascade-deleted with the roll");
}

// kammerz-o0l: deleting a missing roll returns 404 NOT_FOUND, not a no-op 204.
#[tokio::test]
async fn delete_missing_roll_returns_404() {
    let app = open_app().await;

    let res = app.oneshot(delete("/api/rolls/999999")).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "Roll 999999 not found");
}

// --- suggest-id must not collide with a surviving same-day roll (kammerz-cg1) ---
#[tokio::test]
async fn suggest_id_skips_surviving_ids_after_delete() {
    let app = open_app().await;

    // suggest_id keys off today's date, so build the YYMMDD prefix the same way.
    let prefix = chrono::Local::now().format("%y%m%d").to_string();

    // Create two same-day rolls, then delete the *first* one.
    let first_pk = create_roll(&app, &format!("{prefix}-1")).await;
    create_roll(&app, &format!("{prefix}-2")).await;
    let res = app
        .clone()
        .oneshot(delete(&format!("/api/rolls/{first_pk}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // The suggestion must advance past the surviving max id (…-2), not reuse it.
    let res = app.oneshot(get("/api/rolls/suggest-id")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let suggestion: String = json_body(res).await;
    assert_eq!(
        suggestion,
        format!("{prefix}-3"),
        "suggest-id derives the suffix from the max existing id, not a row count"
    );
}

// --- Server-side input validation (kammerz-grd) ---

#[tokio::test]
async fn create_roll_rejects_whitespace_roll_id() {
    let app = open_app().await;
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let res = app
        .oneshot(post_json(
            "/api/rolls",
            &json!({ "roll_id": "   ", "camera_id": camera_id, "status": "loaded" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("roll_id"));
}

#[tokio::test]
async fn create_roll_rejects_negative_frame_count() {
    let app = open_app().await;
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let res = app
        .oneshot(post_json(
            "/api/rolls",
            &json!({
                "roll_id": "NEG-FRAMES",
                "camera_id": camera_id,
                "status": "loaded",
                "frame_count": -36
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("frame_count"));
}

#[tokio::test]
async fn update_roll_rejects_whitespace_roll_id() {
    let app = open_app().await;
    let id = create_roll(&app, "ORIG-ID").await;
    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/rolls/{id}"),
            &json!({ "roll_id": "  " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Original roll_id survives.
    let res = app.oneshot(get(&format!("/api/rolls/{id}"))).await.unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(roll["roll_id"], "ORIG-ID");
}
