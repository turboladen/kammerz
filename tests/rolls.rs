mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
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

    let res = app
        .oneshot(get(&format!("/api/rolls/{id}")))
        .await
        .unwrap();
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
