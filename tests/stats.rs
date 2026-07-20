mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, open_app_with_db, post_json};
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn stats_returns_200() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/stats")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let stats: Value = json_body(res).await;
    assert!(stats.is_object(), "stats deserializes into an object");
}

// ADR-0013: the roll distribution is bucketed by derived lifecycle phase, not by
// a stored status — the endpoint emits `rolls_by_phase` (phase-label + count) and
// no longer `rolls_by_status`.
#[tokio::test]
async fn stats_rolls_by_phase_replaces_rolls_by_status() {
    let app = open_app().await;

    // A fresh roll lands in the Shooting phase (group_key 0).
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &json!({ "roll_id": "PHASE-STAT" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app.oneshot(get("/api/stats")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let stats: Value = json_body(res).await;

    assert!(
        stats["rolls_by_status"].is_null(),
        "rolls_by_status is retired"
    );
    let phases = stats["rolls_by_phase"]
        .as_array()
        .expect("rolls_by_phase is an array");
    assert!(!phases.is_empty(), "at least the fresh roll is counted");
    let known = [
        "Shooting",
        "Development",
        "Scanning",
        "Post-processing",
        "Archiving",
        "Done",
    ];
    for item in phases {
        let label = item["label"].as_str().unwrap();
        assert!(known.contains(&label), "unexpected phase label: {label}");
    }
    assert!(
        phases.iter().any(|p| p["label"] == "Shooting"),
        "the fresh roll is bucketed under Shooting"
    );
}

/// Regression (kammerz-4jn) + defense-in-depth: the API now rejects partial dates
/// (ADR-0011), but `/api/stats` must still not 500 if a legacy partial
/// `date_loaded` ever exists in the DB — `STRFTIME('%Y-%m', …)` returns NULL for
/// 'YYYY-MM' (500ing the endpoint) and misparses bare 'YYYY' as a Julian day
/// number ('-4707-06' garbage buckets). Seed the partials directly, bypassing API
/// validation, to keep exercising the stats query's robustness.
#[tokio::test]
async fn stats_survives_partial_date_loaded() {
    use sea_orm::{ActiveModelTrait, Set};
    let (app, db) = open_app_with_db().await;

    // Recent dates so the rolls fall inside the 12-month window.
    let now = chrono::Utc::now();
    let full = now.format("%Y-%m-%d").to_string();
    let year_month = now.format("%Y-%m").to_string();
    let year = now.format("%Y").to_string();

    for (rid, d) in [
        ("STATS-FULL", &full),
        ("STATS-YM", &year_month),
        ("STATS-Y", &year),
    ] {
        entity::roll::ActiveModel {
            roll_id: Set(rid.to_string()),
            date_loaded: Set(Some(d.clone())),
            created_at: Set("2026-05-01 00:00:00".to_string()),
            updated_at: Set("2026-05-01 00:00:00".to_string()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
    }

    let res = app.oneshot(get("/api/stats")).await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "partial dates must not 500 stats"
    );
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
