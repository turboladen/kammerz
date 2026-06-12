//! Brute-force guard for `POST /api/auth/login`.
//!
//! A single shared password reachable over the LAN/VPN is otherwise bounded only
//! by argon2 cost, so we throttle per client IP with `tower-governor` (GCRA). The
//! layer is attached to the login route only (see `routes::create_router`).
//!
//! The per-client key comes from one of two extractors, chosen by
//! `KAMMERZ_TRUST_PROXY` (default off):
//!   - `PeerIpKeyExtractor` (default) reads the `ConnectInfo<SocketAddr>` that
//!     `main.rs` installs via `into_make_service_with_connect_info` — the actual
//!     TCP peer. Correct when clients connect directly (LAN/VPN).
//!   - `SmartIpKeyExtractor` (trust-proxy mode) keys on `X-Forwarded-For` instead,
//!     so clients behind a TLS reverse proxy — which all share the proxy's peer IP
//!     — get independent buckets. It reads the *leftmost parseable* XFF entry, so
//!     enable this ONLY behind a proxy that *replaces* the header with the single
//!     real client IP. An *appending* proxy (e.g. nginx's default
//!     `$proxy_add_x_forwarded_for`) is unsafe: an attacker sends a forged leading
//!     IP, the proxy appends the real one after it, and the limiter keys on the
//!     attacker's value — a fresh bucket per request, defeating the throttle. XFF
//!     is client-supplied and trivially spoofable without an overwriting proxy.
//!     Without trust-proxy mode at all, the proxy's single IP collapses every
//!     client into one bucket, so an attacker hammering login locks the real user
//!     out. (`SmartIpKeyExtractor` also falls back to `x-real-ip` then `Forwarded`
//!     then the peer IP, but XFF takes precedence whenever present.)
//!
//! We deliberately do NOT spawn governor's background `retain_recent()` cleanup
//! task. The keyspace stays bounded by the handful of distinct VPN/LAN peer IPs in
//! the default mode, and — in a correctly-configured trust-proxy deployment (an
//! overwriting proxy, per the contract above) — by that same set of real client
//! IPs. The governor map has no eviction, so a *misconfigured* trust-proxy
//! deployment that admits forged XFF values would grow it one entry per forged IP;
//! but that same misconfiguration already defeats the throttle (see above), so the
//! memory growth is strictly secondary to fixing the proxy. In every supported
//! configuration storage growth is negligible — and the config is built inside
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
/// Crate-internal: only `routes::create_router` consumes it (tests need only the
/// burst size), so it stays off the public API surface.
pub(crate) const LOGIN_REPLENISH_SECONDS: u64 = 10;

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
        // not expected; surface as a generic 500. `AppError::Internal` logs its
        // message server-side and returns an opaque body, so fold in the governor
        // variant to keep the unexpected case diagnosable without leaking detail.
        other @ (GovernorError::UnableToExtractKey | GovernorError::Other { .. }) => {
            AppError::Internal(format!("login rate limiter: {other}")).into_response()
        }
    }
}
