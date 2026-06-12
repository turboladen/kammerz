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
    // Guards that the handler 503s (with the standard error envelope) when its DB
    // probe fails, rather than reporting a healthy 200. Simulated by closing the
    // pool out from under the router so `SELECT 1` errors.
    //
    // NOTE: this does NOT prove the `SELECT 1`-over-`ping()` switch — a closed
    // pool fails `acquire()`, so `ping()` would 503 here too. The genuine
    // ping-vs-query gap (an OPEN pool whose underlying DB file/engine is gone,
    // where ping's worker-thread check still passes) isn't reproducible
    // in-process; that distinction is argued from the sqlx source in the commit.
    let (app, db) = open_app_with_db().await;
    db.close().await.unwrap();

    let res = app.oneshot(get("/api/health")).await.unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");
}
