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

#[tokio::test]
async fn short_query_returns_empty_results() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/search?q=a")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let results: Value = json_body(res).await;
    assert_eq!(results["cameras"].as_array().unwrap().len(), 0);
}
