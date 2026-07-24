use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use serde::Deserialize;

use crate::AppState;
use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::extract::{Json, Path};
use crate::routes::friendly_err;
use crate::services::settings_service::SettingsService;

#[derive(Debug, Deserialize)]
pub struct SetSettingDto {
    pub value: String,
}

/// Settings whose values are secrets (billable credentials). These are
/// write-only over the API: `GET` never returns the stored value — it reports
/// presence via [`SECRET_MASK`] instead. The real value is only ever read
/// server-side (see `resolve_key` in `routes/import.rs`). `PUT` works normally.
const SECRET_KEYS: &[&str] = &["claude_api_key"];

/// Every setting key the app recognizes. `PUT` rejects anything else so the
/// table can't be used as an arbitrary unauthenticated key/value store
/// (kammerz-vlyu.17). Keep in sync with the keys read in `routes/import.rs`
/// (`claude_api_key`, `claude_model`) and the frontend `import` page.
const KNOWN_KEYS: &[&str] = &["claude_api_key", "claude_model"];

/// Sentinel returned by `GET` for a secret key that has a saved value. The
/// frontend treats any non-null response as "a key is saved" and never echoes
/// this value back.
pub const SECRET_MASK: &str = "********";

pub fn router() -> Router<AppState> {
    Router::new().route("/{key}", get(get_setting).put(set_setting))
}

async fn get_setting(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(key): Path<String>,
) -> AppResult<Json<Option<String>>> {
    let value = SettingsService::get_setting(&db, &key).await?;
    if SECRET_KEYS.contains(&key.as_str()) {
        return Ok(Json(
            value
                .filter(|v| !v.is_empty())
                .map(|_| SECRET_MASK.to_string()),
        ));
    }
    Ok(Json(value))
}

async fn set_setting(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(key): Path<String>,
    Json(data): Json<SetSettingDto>,
) -> AppResult<StatusCode> {
    // Only known keys are writable — reject anything else rather than store it
    // verbatim (kammerz-vlyu.17).
    if !KNOWN_KEYS.contains(&key.as_str()) {
        return Err(AppError::UnprocessableEntity(format!(
            "Unknown setting key '{key}'."
        )));
    }
    // Trim per the trim-everywhere convention: a whitespace-only value is stored
    // as empty (which reads back as "not configured" for a secret key, and is
    // treated as unset by `resolve_key`).
    let value = data.value.trim().to_string();
    SettingsService::set_setting(&db, key, value)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("setting", e)))?;
    Ok(StatusCode::NO_CONTENT)
}
