//! Brute-force guard for `POST /api/auth/login`.
//!
//! A single shared password reachable over the LAN/VPN is otherwise bounded only
//! by argon2 cost, so we throttle per client IP with `tower-governor` (GCRA). The
//! layer is attached to the login route only (see `routes::create_router`); the
//! per-IP key comes from `PeerIpKeyExtractor`, which reads the
//! `ConnectInfo<SocketAddr>` that `main.rs` installs via
//! `into_make_service_with_connect_info`.
//!
//! We deliberately do NOT spawn governor's background `retain_recent()` cleanup
//! task: the login keyspace is bounded by the handful of distinct VPN/LAN client
//! IPs, so storage growth is negligible — and the config is built inside
//! `create_router`, which every integration test calls, so a per-build cleanup
//! thread would cost far more than it saves.

use axum::response::{IntoResponse, Response};
use tower_governor::GovernorError;

use crate::error::AppError;

/// Requests allowed in an immediate burst before throttling kicks in. Comfortably
/// covers a legitimate user fat-fingering the password a few times.
pub const LOGIN_BURST_SIZE: u32 = 5;

/// Seconds to replenish one slot once the burst is spent (GCRA period). 1 attempt
/// per 10s sustained — trivial for the password holder, painful for a brute-forcer.
pub const LOGIN_REPLENISH_SECONDS: u64 = 10;

/// Map a `GovernorError` onto the project's standard `{error:{code,message}}`
/// envelope so a throttled login is byte-identical to every other API error (the
/// frontend `request()` helper parses this shape).
pub fn on_governor_error(err: GovernorError) -> Response {
    match err {
        GovernorError::TooManyRequests { headers, .. } => {
            let mut resp = AppError::TooManyRequests.into_response();
            // Carry governor's `Retry-After` (and any rate-limit headers) through.
            if let Some(headers) = headers {
                resp.headers_mut().extend(headers);
            }
            resp
        }
        // In production `ConnectInfo<SocketAddr>` is always present, so these are
        // not expected; surface as a generic 500 rather than leaking limiter detail.
        GovernorError::UnableToExtractKey | GovernorError::Other { .. } => {
            AppError::Internal("rate limiter error".to_string()).into_response()
        }
    }
}
