mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn search_returns_200_with_results_shape() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/search?q=nikon")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let results: Value = json_body(res).await;
    // Result envelope has the six category arrays.
    assert!(results["cameras"].is_array());
    assert!(results["lenses"].is_array());
    assert!(results["film_stocks"].is_array());
    assert!(results["rolls"].is_array());
    assert!(results["shots"].is_array());
    assert!(results["labs"].is_array());
}

// ADR-0013: a roll search hit carries the server-derived activity summary
// (`badge` + `group_key`) so the search UI renders the same phase Badge as every
// other roll list — there is no stored `status` field anymore.
#[tokio::test]
async fn roll_search_hit_carries_derived_activity_fields() {
    let app = open_app().await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/rolls",
            &json!({ "roll_id": "SEARCHME-42" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app.oneshot(get("/api/search?q=SEARCHME")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let results: Value = json_body(res).await;
    let hit = results["rolls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["roll_id"] == "SEARCHME-42")
        .expect("the created roll appears in search results");

    // A fresh roll (no dates, no shots) is in the shooting phase.
    assert_eq!(hit["group_key"], 0);
    assert_eq!(hit["badge"], "Loaded");
    assert!(hit.get("status").is_none(), "compat status is retired");
}

// Regression (kammerz-1ezf review): the search derivation must consume the SAME
// signal set as the canonical roll list. The *_started columns were once
// hardcoded None here, so a mid-scan roll searched as "To scan" while every
// other list showed "Scanning" — same roll, two labels.
#[tokio::test]
async fn roll_search_badge_matches_canonical_derivation_mid_scan() {
    let app = open_app().await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/rolls",
            &json!({ "roll_id": "SEARCHSCAN-7", "date_finished": "2026-01-05", "scan_started": "2026-02-01" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app.oneshot(get("/api/search?q=SEARCHSCAN")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let results: Value = json_body(res).await;
    let hit = results["rolls"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["roll_id"] == "SEARCHSCAN-7")
        .expect("the created roll appears in search results");

    // scan_started (no completion): scanning is in progress → group_key 2 and
    // the in-progress "Scanning" badge, never the waiting "To scan".
    assert_eq!(hit["group_key"], 2);
    assert_eq!(hit["badge"], "Scanning");
}

#[tokio::test]
async fn short_query_returns_empty_results() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/search?q=a")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let results: Value = json_body(res).await;
    assert_eq!(results["cameras"].as_array().unwrap().len(), 0);
}

// The min-length floor counts characters, not bytes (kammerz-vlyu.27): a single
// multibyte glyph (here the 3-byte CJK char 日, percent-encoded) is one character,
// so it must fall below the 2-char floor and short-circuit to empty results — a
// byte-length check would have let its 3 bytes clear the floor and run the scan.
#[tokio::test]
async fn single_multibyte_char_returns_empty_results() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/search?q=%E6%97%A5")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let results: Value = json_body(res).await;
    for key in ["cameras", "lenses", "film_stocks", "rolls", "shots", "labs"] {
        assert_eq!(
            results[key].as_array().unwrap().len(),
            0,
            "a single-character multibyte query must return no {key}"
        );
    }
}
