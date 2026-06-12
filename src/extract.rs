//! Wrapper extractors that route axum's built-in rejections through the shared
//! `{"error":{"code","message"}}` envelope (kammerz-4lr).
//!
//! axum's stock `Json`/`Path`/`Query` extractors reject malformed input with a
//! plain-text body and no `error.code` — so the frontend `client.ts` catch-all
//! surfaces an opaque `statusText` with code `UNKNOWN`, and invalid-enum drift
//! (Rust enum vs the TypeScript union) is painful to diagnose from a phone. These
//! newtypes delegate to the axum extractors but, on rejection, convert into
//! [`AppError::Rejection`] — preserving axum's own status (`rejection.status()`)
//! and useful diagnostic (`rejection.body_text()`, which carries serde's
//! "line N column M" / offending-field text) inside the envelope.
//!
//! Each wrapper deliberately shares the name of the axum extractor it replaces, so
//! a route module only swaps its `use` line — handler signatures are unchanged
//! (`Json(data): Json<Dto>`, `Path(id): Path<i32>`, `Query(p): Query<Q>`). `Json`
//! is also used as a response type throughout the routes, so it additionally
//! implements [`IntoResponse`] by delegating to `axum::Json`.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;

/// Map an axum rejection's status to the envelope `code`. The status is axum's
/// own classification of the request defect: 415 (no `Content-Type:
/// application/json`), 422 (JSON deserialize / invalid-enum data error), 413
/// (body over the size limit), and 400 (malformed JSON syntax, non-numeric path
/// id, missing query param). Anything else — only reachable if axum adds a 5xx
/// rejection variant (e.g. a misconfigured route with no matching path params) —
/// is an internal misconfiguration, not a client error, so it keeps the same
/// `INTERNAL_ERROR` code the rest of the app uses for 5xx.
fn code_for(status: StatusCode) -> &'static str {
    match status {
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "UNSUPPORTED_MEDIA_TYPE",
        StatusCode::UNPROCESSABLE_ENTITY => "VALIDATION_ERROR",
        StatusCode::PAYLOAD_TOO_LARGE => "PAYLOAD_TOO_LARGE",
        StatusCode::BAD_REQUEST => "BAD_REQUEST",
        s if s.is_server_error() => "INTERNAL_ERROR",
        _ => "BAD_REQUEST",
    }
}

/// Build an [`AppError::Rejection`] from any axum rejection that exposes
/// `status()` + `body_text()` (all of `JsonRejection`/`PathRejection`/
/// `QueryRejection` do). The body text is a request-shape complaint, not a leaked
/// internal, so it is safe to surface verbatim as the user-facing message.
fn into_app_error(status: StatusCode, body_text: String) -> AppError {
    AppError::Rejection {
        status,
        code: code_for(status),
        message: body_text,
    }
}

/// Drop-in for `axum::Json`: extracts a JSON body, and on rejection returns the
/// envelope instead of plain text. Also serializes responses (delegating to
/// `axum::Json`) since routes use `Json(value)` as a return type too.
pub struct Json<T>(pub T);

impl<T, S> FromRequest<S> for Json<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Json(value)),
            Err(rejection) => Err(into_app_error(rejection.status(), rejection.body_text())),
        }
    }
}

impl<T> IntoResponse for Json<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// Drop-in for `axum::extract::Path`: parses path params, and on rejection (e.g.
/// a non-numeric id for an `i32` param) returns the envelope instead of plain
/// text.
pub struct Path<T>(pub T);

impl<T, S> FromRequestParts<S> for Path<T>
where
    axum::extract::Path<T>: FromRequestParts<S, Rejection = PathRejection>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Path::<T>::from_request_parts(parts, state).await {
            Ok(axum::extract::Path(value)) => Ok(Path(value)),
            Err(rejection) => Err(into_app_error(rejection.status(), rejection.body_text())),
        }
    }
}

/// Drop-in for `axum::extract::Query`: parses the query string, and on rejection
/// (e.g. a missing required param) returns the envelope instead of plain text.
pub struct Query<T>(pub T);

impl<T, S> FromRequestParts<S> for Query<T>
where
    axum::extract::Query<T>: FromRequestParts<S, Rejection = QueryRejection>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Query::<T>::from_request_parts(parts, state).await {
            Ok(axum::extract::Query(value)) => Ok(Query(value)),
            Err(rejection) => Err(into_app_error(rejection.status(), rejection.body_text())),
        }
    }
}
