use axum::http::StatusCode;
use serde_json::Value;
use tower::ServiceExt;

mod common;
use common::{app_with_password, get, json_body, open_app};

#[tokio::test]
async fn health_reports_ok_and_version() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/health")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body: Value = json_body(res).await;
    assert_eq!(body["ok"], true);
    // Integration tests compile as part of the `kammerz` package, so this is
    // the same version the binary embeds — guards against the field being
    // dropped or drifting from Cargo.toml.
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn health_is_public_when_password_set() {
    // /api/health is intentionally outside RequireAuth: deployment probes
    // (systemd, CI readiness loop, release smoke checks) must work unauthenticated.
    let app = app_with_password("pw").await;
    let res = app.oneshot(get("/api/health")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body: Value = json_body(res).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}
