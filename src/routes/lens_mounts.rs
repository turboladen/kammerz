use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::Set;
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::routes::friendly_err;
use crate::services::lens_mount_service::LensMountService;
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
    LensMountService::list_all(&db)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn create(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Json(data): Json<CreateLensMountDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = lens_mount::ActiveModel {
        name: Set(data.name),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let res = LensMountService::create(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lens mount", e)))?;
    Ok((StatusCode::CREATED, Json(res.id)))
}
