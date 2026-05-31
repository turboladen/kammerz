use axum::http::StatusCode;
use tower::ServiceExt;

mod common;
use common::{app_with_password, get, open_app};

#[tokio::test]
async fn business_routes_require_auth_when_password_set() {
    let app = app_with_password("pw").await;
    let res = app.oneshot(get("/api/cameras")).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn business_routes_open_when_no_password() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/cameras")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
