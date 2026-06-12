//! ETag / 304 revalidation for embedded assets (kammerz-jax).
//!
//! `main.rs::serve_spa` delegates every embedded-asset response to
//! `kammerz::spa::asset_response`, which attaches a strong, sha256-derived ETag
//! and answers a matching `If-None-Match` with `304 Not Modified`. These tests
//! drive that function directly through a one-route in-process router (never
//! binding a port), so they exercise the exact code `serve_spa` runs without
//! depending on the embedded `frontend/build` contents (wiped from a fresh tree).
//!
//! The synthetic asset stands in for a real non-immutable asset (a font /
//! `index.html` / favicon): stable URL, `no-cache`, hence the validator is what
//! lets the browser skip the re-download.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use http_body_util::BodyExt;
use kammerz::compression::compression_layer;
use kammerz::spa::asset_response;
use tower::ServiceExt;

/// The bytes and content type of the fake asset under test.
const ASSET_BYTES: &[u8] = b"\x00\x01\x02 fake woff2 payload, not really compressed";
const ASSET_MIME: &str = "font/woff2";

/// A fixed, non-trivial sha256 so the ETag is deterministic across runs.
fn asset_sha256() -> [u8; 32] {
    let mut h = [0u8; 32];
    for (i, b) in h.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7).wrapping_add(3);
    }
    h
}

/// Shared per-route config so the handler can echo the request path policy.
#[derive(Clone)]
struct AssetCfg {
    path: &'static str,
}

/// One handler that serves the synthetic asset through `asset_response`, exactly
/// as `serve_spa` does for a real embedded file.
async fn serve(State(cfg): State<Arc<AssetCfg>>, headers: HeaderMap) -> Response<Body> {
    asset_response(
        &headers,
        cfg.path,
        std::borrow::Cow::Borrowed(ASSET_BYTES),
        &asset_sha256(),
        ASSET_MIME,
    )
}

/// Router serving the asset at `/asset`, with `path` controlling the
/// `Cache-Control` policy `asset_response` selects.
fn asset_app(path: &'static str) -> Router {
    Router::new()
        .route("/asset", get(serve))
        .with_state(Arc::new(AssetCfg { path }))
}

fn get_req(inm: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().uri("/asset");
    if let Some(v) = inm {
        b = b.header(IF_NONE_MATCH, v);
    }
    b.body(Body::empty()).unwrap()
}

/// Cold load: a plain GET returns 200 with the body, a content type, a
/// revalidating Cache-Control, and the strong ETag the next request will echo.
#[tokio::test]
async fn first_get_returns_body_and_etag() {
    let app = asset_app("fonts/dm-sans-latin.woff2");
    let res = app.oneshot(get_req(None)).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let etag = res
        .headers()
        .get(ETAG)
        .expect("a strong ETag must be present on the 200")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        etag.starts_with('"') && etag.ends_with('"'),
        "ETag must be a quoted strong tag, got {etag}"
    );
    assert_eq!(res.headers().get(CONTENT_TYPE).unwrap(), ASSET_MIME);
    assert_eq!(res.headers().get(CACHE_CONTROL).unwrap(), "no-cache");

    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), ASSET_BYTES, "200 must carry the full body");
}

/// Warm load: re-requesting with the ETag the server just handed out yields a
/// 304 with an empty body but the validator and Cache-Control retained.
#[tokio::test]
async fn matching_if_none_match_returns_304_empty_body() {
    // Capture the ETag from a first request.
    let etag = {
        let app = asset_app("fonts/dm-sans-latin.woff2");
        let res = app.oneshot(get_req(None)).await.unwrap();
        res.headers()
            .get(ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    };

    let app = asset_app("fonts/dm-sans-latin.woff2");
    let res = app.oneshot(get_req(Some(&etag))).await.unwrap();

    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(
        res.headers().get(ETAG).unwrap().to_str().unwrap(),
        etag,
        "304 must retain the ETag (RFC 9110 §15.4.5)"
    );
    assert_eq!(
        res.headers().get(CACHE_CONTROL).unwrap(),
        "no-cache",
        "304 must retain Cache-Control so the client refreshes freshness"
    );

    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(body.is_empty(), "a 304 must omit the body");
}

/// A stale/foreign validator does not match — the client gets a fresh 200 + body.
#[tokio::test]
async fn mismatched_if_none_match_returns_full_body() {
    let app = asset_app("fonts/dm-sans-latin.woff2");
    let res = app
        .oneshot(get_req(Some(
            "\"0000000000000000000000000000000000000000000000000000000000000000\"",
        )))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), ASSET_BYTES);
}

/// `If-None-Match: *` matches any current representation -> 304.
#[tokio::test]
async fn star_if_none_match_returns_304() {
    let app = asset_app("fonts/dm-sans-latin.woff2");
    let res = app.oneshot(get_req(Some("*"))).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);
}

/// A weak validator (`W/"..."`) still matches our strong tag under the weak
/// comparison `If-None-Match` requires.
#[tokio::test]
async fn weak_validator_matches() {
    let etag = {
        let app = asset_app("fonts/dm-sans-latin.woff2");
        let res = app.oneshot(get_req(None)).await.unwrap();
        res.headers()
            .get(ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    };
    let weak = format!("W/{etag}");

    let app = asset_app("fonts/dm-sans-latin.woff2");
    let res = app.oneshot(get_req(Some(&weak))).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);
}

/// The content-hashed bundle keeps its one-year immutable Cache-Control, and now
/// also carries an ETag (harmless: `immutable` already tells the browser not to
/// revalidate, so the validator is only used if the cache is cleared).
#[tokio::test]
async fn immutable_asset_keeps_immutable_cache_and_gets_etag() {
    let app = asset_app("_app/immutable/chunks/abc123.js");
    let res = app.oneshot(get_req(None)).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(CACHE_CONTROL).unwrap(),
        "public, max-age=31536000, immutable",
        "_app/immutable/* must keep the immutable policy"
    );
    assert!(
        res.headers().get(ETAG).is_some(),
        "an ETag on an immutable asset is fine"
    );
}

/// A 304 flows cleanly through the outer compression layer (nothing to encode on
/// an empty body), so revalidation works in the real layered server.
#[tokio::test]
async fn not_modified_passes_through_compression_layer() {
    let etag = {
        let app = asset_app("fonts/dm-sans-latin.woff2");
        let res = app.oneshot(get_req(None)).await.unwrap();
        res.headers()
            .get(ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    };

    let app = asset_app("fonts/dm-sans-latin.woff2").layer(compression_layer());
    let req = Request::builder()
        .uri("/asset")
        .header(IF_NONE_MATCH, &etag)
        .header("accept-encoding", "gzip, br")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();

    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(body.is_empty(), "304 stays body-less through compression");
}
