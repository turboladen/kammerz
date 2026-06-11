mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn stats_returns_200() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/stats")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let stats: Value = json_body(res).await;
    assert!(stats.is_object(), "stats deserializes into an object");
}

/// Create a roll with the given `date_loaded` on a seeded camera.
async fn create_roll_loaded(app: &axum::Router, roll_id: &str, date_loaded: &str) {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let payload = json!({
        "roll_id": roll_id,
        "camera_id": camera_id,
        "status": "loaded",
        "date_loaded": date_loaded
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
}

/// Regression test for kammerz-4jn: partial `date_loaded` values (which
/// validate.rs accepts) used to break `/api/stats` — `STRFTIME('%Y-%m', …)`
/// returns NULL for 'YYYY-MM' (500ing the whole endpoint) and misparses bare
/// 'YYYY' as a Julian day number ('-4707-06' garbage buckets).
#[tokio::test]
async fn stats_survives_partial_date_loaded() {
    let app = open_app().await;

    // Recent dates so the rolls fall inside the 12-month window.
    let now = chrono::Utc::now();
    let full = now.format("%Y-%m-%d").to_string();
    let year_month = now.format("%Y-%m").to_string();
    let year = now.format("%Y").to_string();

    create_roll_loaded(&app, "STATS-FULL", &full).await;
    create_roll_loaded(&app, "STATS-YM", &year_month).await;
    create_roll_loaded(&app, "STATS-Y", &year).await;

    let res = app.oneshot(get("/api/stats")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "partial dates must not 500 stats");
    let stats: Value = json_body(res).await;

    let months = stats["rolls_per_month"]
        .as_array()
        .expect("rolls_per_month is an array");

    // The full date and the YYYY-MM date both land in the current month bucket.
    let current = months
        .iter()
        .find(|m| m["month"] == year_month.as_str())
        .expect("current month bucket present");
    assert_eq!(current["count"], 2, "full + YYYY-MM rolls bucket together");

    // The year-only roll is skipped (no month to bucket by) and never emits a
    // garbage Julian-day bucket like '-4707-06'.
    for m in months {
        let label = m["month"].as_str().expect("month is a non-null string");
        assert!(
            label.len() == 7 && &label[4..5] == "-",
            "month buckets are well-formed YYYY-MM, got {label:?}"
        );
    }
}
