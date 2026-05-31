use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_sessions::Session;

use crate::auth::middleware::{clear_session, is_authed, set_authed};
use crate::auth::password::verify_password;
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

pub async fn login(
    State(config): State<AppConfig>,
    session: Session,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<Value>> {
    match &config.password_hash {
        // Open mode: any login succeeds (and is unnecessary).
        None => {
            set_authed(&session).await?;
            Ok(Json(json!({ "authenticated": true })))
        }
        Some(hash) => {
            if verify_password(&body.password, hash) {
                session
                    .cycle_id()
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                set_authed(&session).await?;
                Ok(Json(json!({ "authenticated": true })))
            } else {
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
    let authed = config.password_hash.is_none() || is_authed(&session).await;
    Json(json!({ "authenticated": authed, "auth_required": config.password_hash.is_some() }))
}
