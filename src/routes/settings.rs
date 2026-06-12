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
    SettingsService::set_setting(&db, key, data.value)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("setting", e)))?;
    Ok(StatusCode::NO_CONTENT)
}
