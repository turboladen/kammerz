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
// a stored status — the endpoint emits `rolls_by_phase` as {group_key, count}
// buckets (NO label strings: the frontend's PHASE_META owns labels/colors, so
// there is no cross-language string contract to drift) and no `rolls_by_status`.
#[tokio::test]
async fn stats_rolls_by_phase_replaces_rolls_by_status() {
    let app = open_app().await;

    // A fresh roll lands in the Shooting phase (group_key 0). A second roll with
    // only a scan-START date exercises the *_started signals: development is
    // implicitly done via the tail date, scanning is unfinished → group_key 2 —
    // the case a partial signal set used to misbucket under Development.
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &json!({ "roll_id": "PHASE-STAT" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/rolls",
            &json!({ "roll_id": "PHASE-STAT-SCANNING", "scan_started": "2026-02-01" }),
        ))
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
    assert!(!phases.is_empty(), "at least the fresh rolls are counted");
    for item in phases {
        assert!(
            item.get("label").is_none(),
            "phase buckets carry no label key — labels live only in the frontend's PHASE_META"
        );
        let gk = item["group_key"].as_i64().expect("group_key is an integer");
        assert!((0..=5).contains(&gk), "group_key {gk} out of range");
        assert!(item["count"].as_i64().unwrap() >= 1);
    }
    let count_of = |gk: i64| {
        phases
            .iter()
            .find(|p| p["group_key"] == gk)
            .and_then(|p| p["count"].as_i64())
            .unwrap_or(0)
    };
    assert!(
        count_of(0) >= 1,
        "the fresh roll is bucketed under Shooting"
    );
    assert!(
        count_of(2) >= 1,
        "the scan-started roll is bucketed under Scanning, not Development"
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
