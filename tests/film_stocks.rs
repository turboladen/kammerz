mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json, put_json};
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

// kammerz-o0l: deleting a missing film stock returns 404 NOT_FOUND, not a no-op 204.
#[tokio::test]
async fn delete_missing_film_stock_returns_404() {
    let app = open_app().await;

    let res = app
        .oneshot(delete("/api/film-stocks/999999"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "Film stock 999999 not found");
}

// --- Server-side input validation (kammerz-grd) ---

/// Create a valid film stock and return its id.
async fn create_film_stock(app: &axum::Router, name: &str) -> i32 {
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/film-stocks",
            &json!({
                "brand": "Valid",
                "name": name,
                "format": "135",
                "stock_type": "bw-negative"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

#[tokio::test]
async fn create_film_stock_rejects_whitespace_name() {
    let app = open_app().await;
    let res = app
        .oneshot(post_json(
            "/api/film-stocks",
            &json!({
                "brand": "Brandy",
                "name": "   ",
                "format": "135",
                "stock_type": "bw-negative"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("name"));
}

#[tokio::test]
async fn create_film_stock_rejects_negative_iso() {
    let app = open_app().await;
    let res = app
        .oneshot(post_json(
            "/api/film-stocks",
            &json!({
                "brand": "Brandy",
                "name": "Negative ISO",
                "format": "135",
                "stock_type": "bw-negative",
                "iso": -100
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("iso"));
}

#[tokio::test]
async fn update_film_stock_rejects_negative_exposure_count() {
    let app = open_app().await;
    let id = create_film_stock(&app, "Patchstock").await;
    let res = app
        .oneshot(put_json(
            &format!("/api/film-stocks/{id}"),
            &json!({ "exposure_count": -1 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
