use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::Set;
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::patch::now_string;
use crate::routes::friendly_err;
use crate::services::lens_mount_service::LensMountService;
use crate::validate::require_nonempty;
use crate::AppState;
use entity::lens_mount;

#[derive(Debug, Deserialize)]
pub struct CreateLensMountDto {
    pub name: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list).post(create))
}

async fn list(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<Vec<lens_mount::Model>>> {
    Ok(Json(LensMountService::list_all(&db).await?))
}

async fn create(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Json(data): Json<CreateLensMountDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    // Trim + require non-empty: the raw `Set(data.name)` previously stored
    // untrimmed input, so `"  Fixed Lens "` would defeat the name-based
    // fixed-lens detection convention (see CLAUDE.md).
    let name = require_nonempty("name", &data.name)?;

    let now = now_string();
    let model = lens_mount::ActiveModel {
        name: Set(name),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let res = LensMountService::create(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lens mount", e)))?;
    Ok((StatusCode::CREATED, Json(res.id)))
}
