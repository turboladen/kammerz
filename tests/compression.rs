//! HTTP response compression behaviour (kammerz-5n3).
//!
//! `main.rs` wraps the router (API routes + the SPA fallback) in a
//! `CompressionLayer`. These tests rebuild that same outer layer over the shared
//! in-process router and drive it with `ServiceExt::oneshot`, asserting the
//! `Accept-Encoding` negotiation end to end:
//!   - a client advertising gzip gets a `content-encoding: gzip` JSON response,
//!   - a client advertising only brotli gets `br`,
//!   - a client that omits `Accept-Encoding` gets an untouched identity response
//!     (the path the deploy health grep and a bare `curl` take).
//!
//! The compressible target is `GET /api/lens-mounts`: migrations seed it, so the
//! JSON array comfortably clears the default `SizeAbove(32)` predicate floor.

mod common;

use axum::http::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get as get_route;
use axum::Router;
use common::{get, open_app};
use http_body_util::BodyExt;
use tower::ServiceExt;
use tower_http::compression::predicate::{NotForContentType, Predicate};
use tower_http::compression::{CompressionLayer, DefaultPredicate};

/// The exact compression layer `main.rs` mounts: the default predicate with
/// `font/woff2` additionally excluded. Kept in sync with the server by mirroring
/// its `compress_when` argument.
fn compression_layer(
) -> CompressionLayer<tower_http::compression::predicate::And<DefaultPredicate, NotForContentType>>
{
    CompressionLayer::new()
        .compress_when(DefaultPredicate::new().and(NotForContentType::const_new("font/woff2")))
}

/// Mirror of `main.rs`: the shared router with the outermost compression layer.
async fn compressed_app() -> Router {
    open_app().await.layer(compression_layer())
}

/// Build a GET request with an explicit `Accept-Encoding`.
fn get_accepting(path: &str, accept_encoding: &str) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .uri(path)
        .header(ACCEPT_ENCODING, accept_encoding)
        .body(axum::body::Body::empty())
        .unwrap()
}

#[tokio::test]
async fn gzip_accept_yields_gzip_encoded_body() {
    let app = compressed_app().await;
    let res = app
        .oneshot(get_accepting("/api/lens-mounts", "gzip"))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(CONTENT_ENCODING).unwrap(),
        "gzip",
        "a gzip-capable client should get a gzip-encoded response"
    );

    // The framed body is the gzip stream, not the raw JSON — confirm the magic
    // bytes so we know we actually compressed rather than mislabelled identity.
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        body.get(..2),
        Some(&[0x1f, 0x8b][..]),
        "gzip magic number (body should be a non-empty gzip stream)"
    );
}

#[tokio::test]
async fn brotli_accept_yields_brotli_encoded_body() {
    let app = compressed_app().await;
    let res = app
        .oneshot(get_accepting("/api/lens-mounts", "br"))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(CONTENT_ENCODING).unwrap(),
        "br",
        "a brotli-only client should get a brotli-encoded response"
    );
}

#[tokio::test]
async fn no_accept_encoding_yields_identity() {
    // The deploy health grep and a bare `curl` send no Accept-Encoding; they must
    // receive an unencoded body they can read directly.
    let app = compressed_app().await;
    let res = app.oneshot(get("/api/lens-mounts")).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers().get(CONTENT_ENCODING).is_none(),
        "without Accept-Encoding the response must stay identity-encoded"
    );

    // And the identity body is still valid JSON the existing tests rely on.
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let mounts: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(mounts.as_array().is_some_and(|a| !a.is_empty()));
}

#[tokio::test]
async fn woff2_is_left_uncompressed_even_when_gzip_and_br_accepted() {
    // woff2 is an already-compressed container; re-compressing it is CPU spent for
    // negative gain. The custom predicate must exclude it. A plain text body of
    // the same length on the same router still compresses, proving the response
    // was large enough to clear SizeAbove and that only the content-type spared it.
    let big = "a".repeat(4096);
    let woff2_body = big.clone();
    let text_body = big;
    let app = Router::new()
        .route(
            "/font",
            get_route(
                || async move { ([(CONTENT_TYPE, "font/woff2")], woff2_body).into_response() },
            ),
        )
        .route(
            "/text",
            get_route(
                || async move { ([(CONTENT_TYPE, "text/plain")], text_body).into_response() },
            ),
        )
        .layer(compression_layer());

    let res = app
        .clone()
        .oneshot(get_accepting("/font", "gzip, br"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        res.headers().get(CONTENT_ENCODING).is_none(),
        "font/woff2 must not be compressed"
    );

    let res = app
        .oneshot(get_accepting("/text", "gzip, br"))
        .await
        .unwrap();
    assert!(
        res.headers().get(CONTENT_ENCODING).is_some(),
        "a same-size text body should still compress (proves the woff2 skip is content-type-driven, not size)"
    );
}
