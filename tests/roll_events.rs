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
        .oneshot(put_json(&format!("/api/rolls/{id}"), &json!({ "status": "shooting" })))
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
        .oneshot(put_json(&format!("/api/rolls/{id}"), &json!({ "status": "shooting" })))
        .await
        .unwrap();
    let events = events_for(&app, id).await;
    // Most recent (status_changed) comes before the initial roll_loaded.
    assert_eq!(events.first().unwrap()["event_type"], "status_changed");
    assert_eq!(events.last().unwrap()["event_type"], "roll_loaded");
}
