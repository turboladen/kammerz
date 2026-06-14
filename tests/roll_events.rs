mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json, put_json};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Create a roll on a seeded camera; return its id.
async fn create_roll(app: &axum::Router, roll_id: &str, status: &str) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;
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

async fn events_for(app: &axum::Router, roll_id: i32) -> Vec<Value> {
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_id}/detail")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let detail: Value = json_body(res).await;
    detail["events"].as_array().cloned().unwrap_or_default()
}

#[tokio::test]
async fn detail_includes_events_array() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-1", "loaded").await;
    // The events key must exist and be an array (creating a roll logs roll_loaded).
    let events = events_for(&app, id).await;
    assert!(
        events.iter().any(|e| e["event_type"] == "roll_loaded"),
        "expected a roll_loaded event, got: {events:?}"
    );
}

#[tokio::test]
async fn manual_status_change_logs_event() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-2", "loaded").await;

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/rolls/{id}"),
            &json!({ "status": "shooting" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let events = events_for(&app, id).await;
    let sc = events
        .iter()
        .find(|e| e["event_type"] == "status_changed")
        .expect("expected a status_changed event");
    assert_eq!(sc["from_status"], "loaded");
    assert_eq!(sc["to_status"], "shooting");
}

#[tokio::test]
async fn events_are_newest_first() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-3", "loaded").await;
    app.clone()
        .oneshot(put_json(
            &format!("/api/rolls/{id}"),
            &json!({ "status": "shooting" }),
        ))
        .await
        .unwrap();
    let events = events_for(&app, id).await;
    // Both events land in the same second, so this ordering relies on the
    // `id DESC` tiebreak in `list_for_roll` (occurred_at DESC, id DESC) — the
    // later-inserted status_changed has the higher id and sorts first.
    assert_eq!(events.first().unwrap()["event_type"], "status_changed");
    assert_eq!(events.last().unwrap()["event_type"], "roll_loaded");
}

// ---------------------------------------------------------------------------
// Task 6: shot events
// ---------------------------------------------------------------------------

async fn first_shot(app: &axum::Router, roll_id: i32, frame: &str) -> i32 {
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/shots",
            &json!({ "roll_id": roll_id, "frame_number": frame, "date": "2026-05-02" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

#[tokio::test]
async fn logging_a_shot_logs_shot_and_autosync_events() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-4", "loaded").await;
    let shot_id = first_shot(&app, id, "1").await;

    let events = events_for(&app, id).await;
    let shot_ev = events
        .iter()
        .find(|e| e["event_type"] == "shot_logged")
        .expect("expected a shot_logged event");
    assert_eq!(shot_ev["ref_kind"], "shot");
    assert_eq!(shot_ev["ref_id"], shot_id);

    // First shot auto-advances loaded → shooting, which must also be logged.
    assert!(
        events.iter().any(|e| e["event_type"] == "status_changed"
            && e["from_status"] == "loaded"
            && e["to_status"] == "shooting"),
        "expected auto-sync status_changed loaded→shooting, got: {events:?}"
    );
}

// ---------------------------------------------------------------------------
// Task 7: development events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn creating_lab_dev_logs_event() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-5", "shot").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": id, "date_dropped_off": "2026-05-10" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let lab_id: i32 = json_body(res).await;

    let events = events_for(&app, id).await;
    let ev = events
        .iter()
        .find(|e| e["event_type"] == "lab_dev_added")
        .expect("expected a lab_dev_added event");
    assert_eq!(ev["ref_kind"], "lab_dev");
    assert_eq!(ev["ref_id"], lab_id);
}
