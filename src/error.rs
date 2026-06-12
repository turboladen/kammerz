use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    NotFound(String),
    /// User-facing message already made friendly (e.g. via friendly_err).
    UnprocessableEntity(String),
    /// Rate limit exceeded (e.g. brute-force guard on the login route).
    TooManyRequests,
    /// A dependency the request needs is unavailable (e.g. the database failed
    /// its liveness check on `/api/health`). Distinct from `Internal` so probes
    /// see a retry-able 503 rather than a 500.
    ServiceUnavailable(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Authentication required".to_string(),
            ),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, "NOT_FOUND", m),
            AppError::UnprocessableEntity(m) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", m)
            }
            AppError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "TOO_MANY_REQUESTS",
                "Too many login attempts. Please wait and try again.".to_string(),
            ),
            AppError::ServiceUnavailable(m) => {
                tracing::error!("service unavailable: {m}");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "SERVICE_UNAVAILABLE",
                    "The service is temporarily unavailable".to_string(),
                )
            }
            AppError::Internal(m) => {
                tracing::error!("internal error: {m}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
        };
        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        AppError::Internal(e.to_string())
    }
}

/// Turn a successful-but-empty lookup (`Option::None`) into a 404.
///
/// Pairs with the `From<DbErr>` impl above so an update/delete handler reads:
/// ```ignore
/// let existing = CameraService::get_by_id(&db, id).await?.or_404("Camera", id)?;
/// ```
pub trait OptionExt<T> {
    fn or_404(self, label: &str, id: i32) -> AppResult<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn or_404(self, label: &str, id: i32) -> AppResult<T> {
        self.ok_or_else(|| AppError::NotFound(format!("{label} {id} not found")))
    }
}

/// `or_404` for use *inside* a transaction closure, whose error channel is typed
/// to `DbErr` rather than `AppError`. Produces `DbErr::RecordNotFound` (same
/// message format as [`OptionExt::or_404`]) so the post-transaction classifier
/// [`crate::routes::friendly_txn_err`] can map it to a 404 instead of a friendly
/// 422. Without this, a missing-row lookup leaked out as a generic `DbErr` and
/// every transactional delete returned 422 for an already-deleted id.
pub trait DbOptionExt<T> {
    fn or_404_db(self, label: &str, id: i32) -> Result<T, sea_orm::DbErr>;
}

impl<T> DbOptionExt<T> for Option<T> {
    fn or_404_db(self, label: &str, id: i32) -> Result<T, sea_orm::DbErr> {
        self.ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("{label} {id} not found")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use serde_json::Value;

    // A wedged/locked/unmounted DB makes `/api/health` fail its liveness ping;
    // the handler maps that to `ServiceUnavailable`. Integration-testing the
    // failure end-to-end would require wedging the single-connection pool from
    // outside, which isn't feasible in-process — so the contract that probes
    // depend on (503 + the `{error:{code,message}}` envelope, generic message,
    // no leaked internals) is pinned here at the `IntoResponse` boundary.
    #[tokio::test]
    async fn service_unavailable_maps_to_503_envelope() {
        let res =
            AppError::ServiceUnavailable("db ping failed: disk I/O error".into()).into_response();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");
        assert_eq!(
            body["error"]["message"],
            "The service is temporarily unavailable"
        );
        // The raw cause must not leak to unauthenticated callers.
        assert!(!body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("disk I/O"));
    }
}
