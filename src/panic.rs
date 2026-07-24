//! Panic-catching layer (kammerz-vlyu.20).
//!
//! A panic in a request handler would otherwise unwind the connection task and
//! drop the socket with a reset — the client sees a broken connection instead of
//! the `{error:{code,message}}` envelope every other failure returns. This layer
//! catches such a panic and maps it to the standard [`AppError::Internal`] 500,
//! so the frontend's `request()` still parses a well-formed error body.
//!
//! There are no runtime `unwrap`/`expect` in the request path (all are
//! startup/statics/tests), so this is defensive — but the net is cheap and keeps
//! the error contract total.
//!
//! Relies on `panic = "unwind"` (the default profile): [`CatchPanicLayer`] uses
//! `catch_unwind` internally, so a future `panic = "abort"` in a Cargo profile
//! would silently defeat it (the process would abort before the panic is caught).

use std::any::Any;

use axum::response::{IntoResponse, Response};
use tower_http::catch_panic::CatchPanicLayer;

use crate::error::AppError;

/// Fn-pointer type of [`handle_panic`], named so [`catch_panic_layer`] can spell
/// its return type. A plain `fn` pointer is `FnMut + Clone`, satisfying
/// tower-http's blanket `ResponseForPanic` impl for functions.
pub type PanicHandler = fn(Box<dyn Any + Send + 'static>) -> Response;

/// Map a caught panic to the standard 500 error envelope.
///
/// The panic payload (a `&str` or `String` from `panic!`) is folded into
/// [`AppError::Internal`], which logs it server-side via `tracing::error!` and
/// returns the generic `{error:{code:"INTERNAL_ERROR",message:"An internal error
/// occurred"}}` body — so the panic text is recorded for diagnosis but never
/// leaks to the client.
fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response {
    let details = err
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| err.downcast_ref::<&str>().copied())
        .unwrap_or("unknown panic");
    AppError::Internal(format!("handler panicked: {details}")).into_response()
}

/// Build the [`CatchPanicLayer`] the server wraps its router in. Shared by
/// `main.rs` and `tests/panic.rs` so the test exercises the real handler rather
/// than a hand copy (mirrors [`crate::compression::compression_layer`]).
pub fn catch_panic_layer() -> CatchPanicLayer<PanicHandler> {
    CatchPanicLayer::custom(handle_panic as PanicHandler)
}
