use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use http_body_util::BodyExt;
use kammerz::auth::rate_limit::LOGIN_BURST_SIZE;
use tower::ServiceExt;

/// Build a login POST carrying a `ConnectInfo<SocketAddr>` extension for the given
/// client IP. In production `into_make_service_with_connect_info` installs this;
/// `oneshot` bypasses that, so the login rate-limiter's `PeerIpKeyExtractor` would
/// otherwise fail to extract a key. Distinct `ip` values are throttled independently.
fn login_req(ip: &str, password: &str) -> Request<Body> {
    login_req_xff(ip, None, password)
}

/// Like [`login_req`] but optionally sets an `X-Forwarded-For` header. `peer_ip`
/// is the TCP peer (installed as `ConnectInfo`, what `PeerIpKeyExtractor` reads);
/// `xff` is the forwarded client IP a proxy would set (what `SmartIpKeyExtractor`
/// reads). Lets a test pin one and vary the other to prove which the limiter keys on.
fn login_req_xff(peer_ip: &str, xff: Option<&str>, password: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json");
    if let Some(xff) = xff {
        builder = builder.header("x-forwarded-for", xff);
    }
    let mut req = builder
        .body(Body::from(
            serde_json::json!({ "password": password }).to_string(),
        ))
        .unwrap();
    let addr: SocketAddr = format!("{peer_ip}:9999").parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(addr));
    req
}

// Helper to build an app with a known password and in-memory DB. `trust_proxy`
// selects the limiter's key extractor (XFF-aware when true).
async fn test_app(password_hash: Option<String>) -> axum::Router {
    test_app_cfg(password_hash, false).await
}

async fn test_app_cfg(password_hash: Option<String>, trust_proxy: bool) -> axum::Router {
    let db = kammerz::db::init("sqlite::memory:").await.unwrap();
    let config = kammerz::config::AppConfig {
        password_hash,
        trust_proxy,
        ..kammerz::config::AppConfig::default()
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
    kammerz::routes::create_router(kammerz::AppState {
        db,
        config,
        db_url: "sqlite::memory:".to_string(),
    })
    .layer(layer)
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
    let res = app.oneshot(login_req("127.0.0.1", "nope")).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_rate_limited_after_burst() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;

    // The burst quota of wrong-password attempts all reach the handler → 401.
    for _ in 0..LOGIN_BURST_SIZE {
        let res = app
            .clone()
            .oneshot(login_req("10.0.0.1", "nope"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // The next attempt (within the replenish window) is throttled → 429, returned
    // through the standard error envelope with a Retry-After header.
    let res = app
        .clone()
        .oneshot(login_req("10.0.0.1", "nope"))
        .await
        .unwrap();
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
async fn login_with_correct_password_succeeds_within_burst() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;

    // A couple of fat-fingered failures (still inside the burst) must not lock the
    // user out: the correct password on a later within-burst attempt still reaches
    // the handler and succeeds.
    for _ in 0..2 {
        let res = app
            .clone()
            .oneshot(login_req("10.0.0.2", "nope"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let res = app.oneshot(login_req("10.0.0.2", "pw")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["authenticated"], true);
}

/// Extract the session cookie the `SessionManagerLayer` sets on a response and
/// return it as a `name=value` pair suitable for a `Cookie` request header. We
/// forward only the first `;`-delimited segment (dropping `HttpOnly`/`Path`/…),
/// and don't hardcode the cookie name — so the thread survives a tower-sessions
/// default-name change.
fn session_cookie(res: &Response<Body>) -> String {
    res.headers()
        .get(SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(';').next())
        .expect("login response should set a session cookie")
        .to_string()
}

/// A plain GET/POST carrying a `Cookie` header (no `ConnectInfo` — only `login`
/// needs the peer IP for rate limiting; `/me` and `/logout` don't).
fn req_with_cookie(method: &str, uri: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

async fn json_body(res: Response<Body>) -> serde_json::Value {
    let body = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn session_cookie_threads_login_me_logout_me() {
    // The whole point: thread the real session cookie through the lifecycle the
    // response-body-only login test never exercised — login mints an authed
    // session, /me carrying its cookie reports authed, and /logout flushes it so
    // the same cookie then reports unauthed. Guards a regression where logout
    // silently fails to clear the session.
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;

    // Log in and capture the session cookie the layer sets.
    let login = app
        .clone()
        .oneshot(login_req("127.0.0.1", "pw"))
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = session_cookie(&login);

    // /me carrying the cookie sees the authed session.
    let me1 = app
        .clone()
        .oneshot(req_with_cookie("GET", "/api/auth/me", &cookie))
        .await
        .unwrap();
    assert_eq!(me1.status(), StatusCode::OK);
    let v = json_body(me1).await;
    assert_eq!(v["authenticated"], true);
    assert_eq!(v["auth_required"], true);

    // Logout flushes the session server-side.
    let logout = app
        .clone()
        .oneshot(req_with_cookie("POST", "/api/auth/logout", &cookie))
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::OK);
    let v = json_body(logout).await;
    assert_eq!(v["authenticated"], false);

    // The SAME cookie now resolves to no session → unauthed. (flush() removed the
    // record from the store, so reusing the login cookie is a valid check.)
    let me2 = app
        .oneshot(req_with_cookie("GET", "/api/auth/me", &cookie))
        .await
        .unwrap();
    assert_eq!(me2.status(), StatusCode::OK);
    let v = json_body(me2).await;
    assert_eq!(v["authenticated"], false);
}

#[tokio::test]
async fn rate_limit_is_per_ip() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;

    // Exhaust the burst for one IP — each priming request still reaches the
    // handler (401), which also guards against a mis-sized burst masking the test.
    for _ in 0..LOGIN_BURST_SIZE {
        let res = app
            .clone()
            .oneshot(login_req("10.0.0.3", "nope"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let blocked = app
        .clone()
        .oneshot(login_req("10.0.0.3", "nope"))
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);

    // A different IP still has its own fresh quota.
    let other = app
        .clone()
        .oneshot(login_req("10.0.0.4", "nope"))
        .await
        .unwrap();
    assert_eq!(other.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn trust_proxy_keys_rate_limit_on_x_forwarded_for() {
    // The reverse-proxy scenario the opt-in exists for: every request arrives from
    // the same proxy peer IP, distinguished only by X-Forwarded-For. In trust-proxy
    // mode the limiter must key on XFF so two real clients get independent buckets —
    // exhausting one must not throttle the other.
    let app = test_app_cfg(
        Some(kammerz::auth::password::hash_password("pw").unwrap()),
        true,
    )
    .await;

    // Exhaust the burst for client A (all share the proxy's peer IP "10.9.9.9").
    for _ in 0..LOGIN_BURST_SIZE {
        let res = app
            .clone()
            .oneshot(login_req_xff("10.9.9.9", Some("203.0.113.1"), "nope"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    let blocked = app
        .clone()
        .oneshot(login_req_xff("10.9.9.9", Some("203.0.113.1"), "nope"))
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);

    // Client B — same proxy peer IP, different XFF — still has a fresh bucket.
    let other = app
        .clone()
        .oneshot(login_req_xff("10.9.9.9", Some("203.0.113.2"), "nope"))
        .await
        .unwrap();
    assert_eq!(
        other.status(),
        StatusCode::UNAUTHORIZED,
        "a distinct X-Forwarded-For must get its own bucket in trust-proxy mode"
    );
}

#[tokio::test]
async fn default_mode_ignores_x_forwarded_for() {
    // Without trust-proxy mode, XFF is client-supplied and must NOT be trusted:
    // varying it cannot mint a fresh bucket. All requests share the peer IP's
    // bucket, so spoofing XFF can't escape the throttle.
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;

    // Exhaust the burst on one peer IP while rotating the (untrusted) XFF header.
    for i in 0..LOGIN_BURST_SIZE {
        let xff = format!("203.0.113.{i}");
        let res = app
            .clone()
            .oneshot(login_req_xff("10.8.8.8", Some(&xff), "nope"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
    // A brand-new XFF value on the same peer IP is still throttled — proof the
    // limiter keyed on the peer IP, not the spoofable header.
    let blocked = app
        .clone()
        .oneshot(login_req_xff("10.8.8.8", Some("203.0.113.250"), "nope"))
        .await
        .unwrap();
    assert_eq!(
        blocked.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "default mode must ignore X-Forwarded-For so a spoofed header can't escape the throttle"
    );
}
