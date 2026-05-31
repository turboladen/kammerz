use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::Set;
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::patch::{double_option, trim, trim_opt};
use crate::routes::friendly_err;
use crate::services::lab_service::LabService;
use crate::AppState;
use entity::lab;

// --- DTOs (moved verbatim from commands/labs.rs) ---

#[derive(Debug, Deserialize)]
pub struct CreateLabDto {
    pub name: String,
    pub location: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateLabDto {
    pub name: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub location: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub website: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(delete_one))
}

// --- Handlers ---

async fn list(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<Vec<lab::Model>>> {
    LabService::list_all(&db)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn get_one(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Option<lab::Model>>> {
    LabService::get_by_id(&db, id)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn create(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Json(data): Json<CreateLabDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = lab::ActiveModel {
        name: trim(data.name),
        location: trim_opt(data.location),
        website: trim_opt(data.website),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let res = LabService::create(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lab", e)))?;
    Ok((StatusCode::CREATED, Json(res.id)))
}

async fn update(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateLabDto>,
) -> AppResult<StatusCode> {
    let existing = LabService::get_by_id(&db, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Lab {id} not found")))?;
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: lab::ActiveModel = existing.into();
    if let Some(v) = data.name {
        model.name = trim(v);
    }
    if let Some(v) = data.location {
        model.location = trim_opt(v);
    }
    if let Some(v) = data.website {
        model.website = trim_opt(v);
    }
    if let Some(v) = data.notes {
        model.notes = trim_opt(v);
    }
    model.updated_at = Set(now);
    LabService::update(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lab", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_one(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    LabService::delete(&db, id)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lab", e)))?;
    Ok(StatusCode::NO_CONTENT)
}
