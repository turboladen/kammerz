mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, post_json, put_json};
use serde_json::{Value, json};
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

#[tokio::test]
async fn update_film_stock_rejects_negative_iso() {
    let app = open_app().await;
    let id = create_film_stock(&app, "Isostock").await;
    let res = app
        .oneshot(put_json(
            &format!("/api/film-stocks/{id}"),
            &json!({ "iso": -50 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn update_film_stock_rejects_whitespace_brand() {
    let app = open_app().await;
    let id = create_film_stock(&app, "Brandstock").await;
    let res = app
        .oneshot(put_json(
            &format!("/api/film-stocks/{id}"),
            &json!({ "brand": "   " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn update_film_stock_rejects_whitespace_name() {
    let app = open_app().await;
    let id = create_film_stock(&app, "Namestock").await;
    let res = app
        .oneshot(put_json(
            &format!("/api/film-stocks/{id}"),
            &json!({ "name": "  " }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// --- Full update + delete + distinct endpoint (kammerz-do4) ---

/// Exercise every setter branch in the film-stock `update` handler in one PUT —
/// no prior test reached the success path (the existing update test rejects at
/// validation before any field is set).
#[tokio::test]
async fn update_film_stock_sets_every_field() {
    let app = open_app().await;
    let id = create_film_stock(&app, "Before 400").await;

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/film-stocks/{id}"),
            &json!({
                "brand": "Afterfilm",
                "name": "Chrome 100",
                "format": "120",
                "exposure_count": 12,
                "stock_type": "color-slide",
                "iso": 100,
                "notes": "push to 200"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .oneshot(get(&format!("/api/film-stocks/{id}")))
        .await
        .unwrap();
    let stock: Value = json_body(res).await;
    assert_eq!(stock["brand"], "Afterfilm");
    assert_eq!(stock["name"], "Chrome 100");
    assert_eq!(stock["format"], "120");
    assert_eq!(stock["exposure_count"], 12);
    assert_eq!(stock["stock_type"], "color-slide");
    assert_eq!(stock["iso"], 100);
    assert_eq!(stock["notes"], "push to 200");
}

#[tokio::test]
async fn delete_film_stock_removes_it() {
    let app = open_app().await;
    let id = create_film_stock(&app, "Deletestock").await;

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/film-stocks/{id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // get_one returns Json<Option<Model>> → 200 with a null body once gone.
    let res = app
        .oneshot(get(&format!("/api/film-stocks/{id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let stock: Value = json_body(res).await;
    assert!(stock.is_null(), "deleted film stock reads back as null");
}

#[tokio::test]
async fn distinct_film_stock_brands_lists_brands_ascending() {
    let app = open_app().await;
    for (brand, name) in [("Zenith", "z"), ("Acme", "a")] {
        let res = app
            .clone()
            .oneshot(post_json(
                "/api/film-stocks",
                &json!({ "brand": brand, "name": name, "format": "135", "stock_type": "bw-negative" }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    let res = app
        .oneshot(get("/api/film-stocks/distinct/brands"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let brands: Vec<String> = json_body(res).await;
    assert!(brands.contains(&"Acme".to_string()) && brands.contains(&"Zenith".to_string()));
    let (ai, zi) = (
        brands.iter().position(|b| b == "Acme").unwrap(),
        brands.iter().position(|b| b == "Zenith").unwrap(),
    );
    assert!(ai < zi, "brands come back ASC-sorted: {brands:?}");
}
