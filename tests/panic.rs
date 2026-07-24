//! Panic-catching layer (kammerz-vlyu.20).
//!
//! A request handler that panics must return the standard `{error:{code,message}}`
//! 500 envelope, not drop the connection with a reset. `main.rs` wraps the router
//! in `kammerz::panic::catch_panic_layer()`; this test drives that same production
//! layer over a synthetic panicking route — the layer and its panic handler are
//! the real code, only the route that panics is a test fixture (mirroring how
//! `tests/compression.rs` exercises the production compression layer).

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use http_body_util::BodyExt;
use kammerz::panic::catch_panic_layer;
use serde_json::Value;
use tower::ServiceExt;

async fn boom() -> &'static str {
    panic!("boom in handler");
}

#[tokio::test]
async fn handler_panic_returns_500_error_envelope() {
    let app = Router::new()
        .route("/boom", get(boom))
        .layer(catch_panic_layer());

    let res = app
        .oneshot(Request::builder().uri("/boom").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], "INTERNAL_ERROR");
    assert_eq!(body["error"]["message"], "An internal error occurred");
    // The panic payload ("boom in handler") is logged server-side but must never
    // leak to the client — scan the whole body, not just the message field.
    assert!(
        !bytes.windows(4).any(|w| w == b"boom"),
        "panic text must not leak into the response body"
    );
}
