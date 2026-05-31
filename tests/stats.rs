mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn stats_returns_200() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/stats")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let stats: Value = json_body(res).await;
    assert!(stats.is_object(), "stats deserializes into an object");
}
