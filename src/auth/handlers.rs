use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_sessions::Session;

use crate::auth::middleware::{clear_session, is_authed, set_authed};
use crate::auth::password::verify_password;
use crate::auth::rate_limit::{Decision, LoginRateLimiter};
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

pub async fn login(
    State(config): State<AppConfig>,
    State(limiter): State<Arc<LoginRateLimiter>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    session: Session,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<Value>> {
    match &config.password_hash {
        // Open mode: any login succeeds (and is unnecessary), so there is nothing
        // to brute-force and no need to throttle.
        None => {
            set_authed(&session).await?;
            Ok(Json(json!({ "authenticated": true })))
        }
        Some(hash) => {
            let ip = peer.ip();
            // Consult the throttle BEFORE verifying: a locked-out IP may not test
            // any password until its window elapses. This is what bounds a
            // brute-forcer's guess rate.
            if let Decision::Locked { retry_after_secs } = limiter.check(ip) {
                return Err(AppError::TooManyRequests { retry_after_secs });
            }
            if verify_password(&body.password, hash) {
                limiter.record_success(ip); // clear the IP's failure budget
                session
                    .cycle_id()
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                set_authed(&session).await?;
                Ok(Json(json!({ "authenticated": true })))
            } else {
                limiter.record_failure(ip);
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
