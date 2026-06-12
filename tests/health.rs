use axum::http::StatusCode;
use serde_json::Value;
use tower::ServiceExt;

mod common;
use common::{app_with_password, get, json_body, open_app, open_app_with_db};

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
    // The exact SHA depends on the checkout; assert the field is a non-empty
    // string so a dropped build.rs env var fails loudly.
    assert!(!body["build"].as_str().unwrap_or("").is_empty());
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

#[tokio::test]
async fn health_reports_503_when_db_is_dead() {
    // The bead's headline scenario: the DB becomes unreachable (file deleted, NAS
    // dir unmounted) while the process stays up. Simulated here by closing the
    // pool out from under the router — the health handler's `SELECT 1` then fails
    // and must surface a 503 with the standard error envelope, NOT a healthy 200.
    // (This is why `ping()` was insufficient: its sqlx-sqlite handler only checks
    // the worker thread is alive and would still report healthy here.)
    let (app, db) = open_app_with_db().await;
    db.close().await.unwrap();

    let res = app.oneshot(get("/api/health")).await.unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");
}
