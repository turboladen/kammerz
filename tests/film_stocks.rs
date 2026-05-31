mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn list_film_stocks_returns_200() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/film-stocks")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let _stocks: Vec<Value> = json_body(res).await;
}

#[tokio::test]
async fn create_then_get_film_stock_roundtrips() {
    let app = open_app().await;

    let payload = json!({
        "brand": "Testfilm",
        "name": "Mono 400",
        "format": "135",
        "exposure_count": 36,
        "stock_type": "bw-negative",
        "iso": 400
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/film-stocks", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let new_id: i32 = json_body(res).await;

    let res = app
        .oneshot(get(&format!("/api/film-stocks/{new_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let stock: Value = json_body(res).await;
    assert_eq!(stock["id"].as_i64().unwrap() as i32, new_id);
    assert_eq!(stock["brand"], "Testfilm");
    assert_eq!(stock["name"], "Mono 400");
}
