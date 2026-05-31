use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

// Helper to build an app with a known password and in-memory DB.
async fn test_app(password_hash: Option<String>) -> axum::Router {
    let db = kammerz::db::init("sqlite::memory:").await.unwrap();
    let config = kammerz::config::AppConfig {
        password_hash,
        anthropic_api_key: None,
        secure_cookies: false,
    };
    // Build router WITH a session layer backed by an in-memory DB. min_connections(1)
    // keeps the in-memory DB alive for the life of the pool.
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .min_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let store = tower_sessions_sqlx_store::SqliteStore::new(pool);
    store.migrate().await.unwrap();
    let layer = tower_sessions::SessionManagerLayer::new(store);
    kammerz::routes::create_router(kammerz::AppState { db, config }).layer(layer)
}

#[tokio::test]
async fn me_reports_unauthed_when_password_set() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["authenticated"], false);
    assert_eq!(v["auth_required"], true);
}

#[tokio::test]
async fn login_with_wrong_password_is_401() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"password":"nope"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
