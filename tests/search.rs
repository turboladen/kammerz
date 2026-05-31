mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app};
use serde_json::Value;
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

#[tokio::test]
async fn short_query_returns_empty_results() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/search?q=a")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let results: Value = json_body(res).await;
    assert_eq!(results["cameras"].as_array().unwrap().len(), 0);
}
