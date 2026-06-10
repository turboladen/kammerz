use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use kammerz::auth::rate_limit::LOGIN_MAX_FAILURES;
use tower::ServiceExt;

/// Build a login POST carrying a `ConnectInfo<SocketAddr>` extension for the given
/// client IP. In production `into_make_service_with_connect_info` installs this;
/// `oneshot` bypasses that, so the login rate-limiter's `PeerIpKeyExtractor` would
/// otherwise fail to extract a key. Distinct `ip` values are throttled independently.
fn login_req(ip: &str, password: &str) -> Request<Body> {
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "password": password }).to_string(),
        ))
        .unwrap();
    let addr: SocketAddr = format!("{ip}:9999").parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(addr));
    req
}

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
    kammerz::routes::create_router(kammerz::AppState::new(db, config)).layer(layer)
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
        .oneshot(login_req("127.0.0.1", "nope"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_locks_out_after_repeated_failures() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;

    // The failure budget of wrong-password attempts all reach the handler → 401.
    for _ in 0..LOGIN_MAX_FAILURES {
        let res = app.clone().oneshot(login_req("10.0.0.1", "nope")).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // The next attempt is locked out → 429, returned through the standard error
    // envelope with a Retry-After header.
    let res = app.clone().oneshot(login_req("10.0.0.1", "nope")).await.unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        res.headers().contains_key("retry-after"),
        "429 should carry a Retry-After header"
    );
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"]["code"], "TOO_MANY_REQUESTS");
}

#[tokio::test]
async fn locked_out_ip_cannot_use_even_the_correct_password() {
    // The throttle is consulted before the password is verified, so once an IP is
    // locked out it cannot log in even with the right password until the window
    // elapses — this is what makes the limit a real brute-force control rather
    // than a cosmetic status-code change.
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;
    for _ in 0..LOGIN_MAX_FAILURES {
        let res = app.clone().oneshot(login_req("10.0.0.5", "nope")).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let res = app.oneshot(login_req("10.0.0.5", "pw")).await.unwrap();
    assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn correct_password_succeeds_and_resets_the_failure_budget() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;

    // Fat-finger up to the brink of lockout, then log in correctly.
    for _ in 0..(LOGIN_MAX_FAILURES - 1) {
        let res = app.clone().oneshot(login_req("10.0.0.2", "nope")).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let res = app.clone().oneshot(login_req("10.0.0.2", "pw")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["authenticated"], true);

    // The success cleared the budget, so a fresh round of failures is allowed
    // again rather than being immediately locked.
    let res = app.oneshot(login_req("10.0.0.2", "nope")).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn repeated_successful_logins_are_not_throttled() {
    // Mirrors the e2e smoke suite: many successful logins from a single IP. Since
    // only failures count toward the limit, these are never throttled.
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;
    for _ in 0..(LOGIN_MAX_FAILURES * 4) {
        let res = app.clone().oneshot(login_req("10.0.0.6", "pw")).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn rate_limit_is_per_ip() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;

    // Exhaust the failure budget for one IP — each priming request still reaches
    // the handler (401), which also guards against a mis-sized budget masking the test.
    for _ in 0..LOGIN_MAX_FAILURES {
        let res = app.clone().oneshot(login_req("10.0.0.3", "nope")).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let blocked = app.clone().oneshot(login_req("10.0.0.3", "nope")).await.unwrap();
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);

    // A different IP still has its own fresh budget.
    let other = app.clone().oneshot(login_req("10.0.0.4", "nope")).await.unwrap();
    assert_eq!(other.status(), StatusCode::UNAUTHORIZED);
}
