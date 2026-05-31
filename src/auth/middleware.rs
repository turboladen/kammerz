use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use tower_sessions::Session;

use crate::config::AppConfig;
use crate::error::AppError;

pub const AUTH_KEY: &str = "kammerz.authed";

pub async fn set_authed(session: &Session) -> Result<(), AppError> {
    session
        .insert(AUTH_KEY, true)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn clear_session(session: &Session) -> Result<(), AppError> {
    session
        .flush()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn is_authed(session: &Session) -> bool {
    session
        .get::<bool>(AUTH_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// Extractor that rejects unauthenticated requests with 401 — UNLESS no
/// password hash is configured (open LAN-trust mode), in which case it passes.
pub struct RequireAuth;

impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
    AppConfig: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let config = AppConfig::from_ref(state);
        if config.password_hash.is_none() {
            return Ok(RequireAuth); // open mode
        }
        let session = parts
            .extensions
            .get::<Session>()
            .ok_or(AppError::Unauthorized)?;
        if is_authed(session).await {
            Ok(RequireAuth)
        } else {
            Err(AppError::Unauthorized)
        }
    }
}
