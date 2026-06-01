use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::routes::friendly_err;
use crate::services::settings_service::SettingsService;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SetSettingDto {
    pub value: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/{key}", get(get_setting).put(set_setting))
}

async fn get_setting(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(key): Path<String>,
) -> AppResult<Json<Option<String>>> {
    Ok(Json(SettingsService::get_setting(&db, &key).await?))
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
