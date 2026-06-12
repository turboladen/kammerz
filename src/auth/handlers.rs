use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_sessions::Session;

use crate::auth::middleware::{clear_session, is_authed, set_authed};
use crate::auth::password::verify_password;
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::extract::Json;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

/// Best-effort client IP for the login audit log, mirroring the rate limiter's
/// keying rule (see `auth::rate_limit`) so the logged and throttled identities
/// agree for a given attempt.
///
/// In trust-proxy mode we follow `SmartIpKeyExtractor`'s precedence: the leftmost
/// parseable `X-Forwarded-For` entry, then `X-Real-IP`, then the peer socket IP.
/// (The limiter also consults a `Forwarded` header between `X-Real-IP` and the peer
/// IP; parsing that RFC 7239 grammar would pull in a dependency just for an audit
/// string, so we skip it — a deployment relying solely on `Forwarded` would see the
/// logged IP fall through to the peer while the limiter keys on the forwarded value.
/// Every common reverse proxy sets `X-Forwarded-For` or `X-Real-IP`.) Without
/// trust-proxy we use the peer IP directly.
///
/// NOTE: like the limiter, the forwarding headers are client-spoofable unless a
/// reverse proxy *overwrites* them, so this string is only trustworthy in a
/// correctly-configured trust-proxy deployment; it is an operator hint, not an
/// authorization input.
fn client_ip(headers: &HeaderMap, peer: SocketAddr, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(ip) = forwarded
                .split(',')
                .find_map(|s| s.trim().parse::<std::net::IpAddr>().ok())
            {
                return ip.to_string();
            }
        }
        if let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
        {
            return ip.to_string();
        }
    }
    peer.ip().to_string()
}

pub async fn login(
    State(config): State<AppConfig>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    session: Session,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<Value>> {
    let ip = client_ip(&headers, peer, config.trust_proxy);
    match &config.password_hash {
        // Open mode: any login succeeds (and is unnecessary).
        None => {
            set_authed(&session).await?;
            tracing::info!(client_ip = %ip, "login succeeded (open mode)");
            Ok(Json(json!({ "authenticated": true })))
        }
        Some(hash) => {
            if verify_password(&body.password, hash) {
                session
                    .cycle_id()
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                set_authed(&session).await?;
                tracing::info!(client_ip = %ip, "login succeeded");
                Ok(Json(json!({ "authenticated": true })))
            } else {
                // Operator signal for brute-force / typo'd-password attempts. We
                // never log the submitted password or its length.
                tracing::warn!(client_ip = %ip, "failed login");
                Err(AppError::Unauthorized)
            }
        }
    }
}

pub async fn logout(session: Session) -> AppResult<Json<Value>> {
    clear_session(&session).await?;
    Ok(Json(json!({ "authenticated": false })))
}

pub async fn me(State(config): State<AppConfig>, session: Session) -> Json<Value> {
    let authed = !config.auth_enabled() || is_authed(&session).await;
    Json(json!({ "authenticated": authed, "auth_required": config.auth_enabled() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer() -> SocketAddr {
        "203.0.113.7:54321".parse().unwrap()
    }

    #[test]
    fn falls_back_to_peer_ip_without_trust_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "198.51.100.9".parse().unwrap());
        // XFF is ignored when trust-proxy is off — the header is client-spoofable.
        assert_eq!(client_ip(&headers, peer(), false), "203.0.113.7");
    }

    #[test]
    fn uses_leftmost_xff_entry_in_trust_proxy_mode() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "198.51.100.9, 203.0.113.1".parse().unwrap(),
        );
        assert_eq!(client_ip(&headers, peer(), true), "198.51.100.9");
    }

    #[test]
    fn falls_back_to_x_real_ip_when_xff_absent() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "198.51.100.9".parse().unwrap());
        assert_eq!(client_ip(&headers, peer(), true), "198.51.100.9");
        // XFF still wins over X-Real-IP when both are present (limiter precedence).
        headers.insert("x-forwarded-for", "192.0.2.5".parse().unwrap());
        assert_eq!(client_ip(&headers, peer(), true), "192.0.2.5");
    }

    #[test]
    fn trust_proxy_falls_back_to_peer_when_headers_unparseable() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "not-an-ip".parse().unwrap());
        headers.insert("x-real-ip", "also-not-an-ip".parse().unwrap());
        assert_eq!(client_ip(&headers, peer(), true), "203.0.113.7");
        // And when the headers are absent entirely.
        assert_eq!(client_ip(&HeaderMap::new(), peer(), true), "203.0.113.7");
    }
}
