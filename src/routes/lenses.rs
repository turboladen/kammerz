use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::Set;
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult, OptionExt};
use crate::patch::{double_option, now_string, trim, trim_opt};
use crate::routes::friendly_err;
use crate::services::lens_service::LensService;
use crate::AppState;
use entity::lens;

// --- DTOs (moved verbatim from commands/lenses.rs) ---

#[derive(Debug, Deserialize)]
pub struct CreateLensDto {
    pub brand: String,
    pub lens_mount_id: i32,
    pub lens_system: Option<String>,
    pub model: Option<String>,
    pub focal_length: Option<String>,
    pub max_aperture: Option<String>,
    pub min_aperture: Option<String>,
    pub filter_thread_front_mm: Option<i32>,
    pub filter_thread_rear_mm: Option<i32>,
    pub serial_number: Option<String>,
    pub date_purchased: Option<String>,
    pub purchased_from: Option<String>,
    pub date_sold: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateLensDto {
    pub brand: Option<String>,
    pub lens_mount_id: Option<i32>,
    #[serde(deserialize_with = "double_option")]
    pub lens_system: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub model: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub focal_length: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub max_aperture: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub min_aperture: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub filter_thread_front_mm: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub filter_thread_rear_mm: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub serial_number: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_purchased: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub purchased_from: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_sold: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/distinct/brands", get(distinct_brands))
        .route("/distinct/systems", get(distinct_systems))
        .route("/{id}", get(get_one).put(update).delete(delete_one))
        .route("/{id}/cameras", get(cameras_for_lens))
}

// --- Handlers ---

async fn list(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<Vec<lens::Model>>> {
    Ok(Json(LensService::list_all(&db).await?))
}

async fn get_one(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Option<lens::Model>>> {
    Ok(Json(LensService::get_by_id(&db, id).await?))
}

async fn create(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Json(data): Json<CreateLensDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    let now = now_string();
    let model = lens::ActiveModel {
        brand: trim(data.brand),
        lens_mount_id: Set(data.lens_mount_id),
        lens_system: trim_opt(data.lens_system),
        model: trim_opt(data.model),
        focal_length: trim_opt(data.focal_length),
        max_aperture: trim_opt(data.max_aperture),
        min_aperture: trim_opt(data.min_aperture),
        filter_thread_front_mm: Set(data.filter_thread_front_mm),
        filter_thread_rear_mm: Set(data.filter_thread_rear_mm),
        serial_number: trim_opt(data.serial_number),
        date_purchased: trim_opt(data.date_purchased),
        purchased_from: trim_opt(data.purchased_from),
        date_sold: trim_opt(data.date_sold),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let res = LensService::create(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lens", e)))?;
    Ok((StatusCode::CREATED, Json(res.id)))
}

async fn update(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateLensDto>,
) -> AppResult<StatusCode> {
    let existing = LensService::get_by_id(&db, id).await?.or_404("Lens", id)?;
    let now = now_string();
    let mut model: lens::ActiveModel = existing.into();
    if let Some(v) = data.brand {
        model.brand = trim(v);
    }
    if let Some(v) = data.lens_mount_id {
        model.lens_mount_id = Set(v);
    }
    if let Some(v) = data.lens_system {
        model.lens_system = trim_opt(v);
    }
    if let Some(v) = data.model {
        model.model = trim_opt(v);
    }
    if let Some(v) = data.focal_length {
        model.focal_length = trim_opt(v);
    }
    if let Some(v) = data.max_aperture {
        model.max_aperture = trim_opt(v);
    }
    if let Some(v) = data.min_aperture {
        model.min_aperture = trim_opt(v);
    }
    if let Some(v) = data.filter_thread_front_mm {
        model.filter_thread_front_mm = Set(v);
    }
    if let Some(v) = data.filter_thread_rear_mm {
        model.filter_thread_rear_mm = Set(v);
    }
    if let Some(v) = data.serial_number {
        model.serial_number = trim_opt(v);
    }
    if let Some(v) = data.date_purchased {
        model.date_purchased = trim_opt(v);
    }
    if let Some(v) = data.purchased_from {
        model.purchased_from = trim_opt(v);
    }
    if let Some(v) = data.date_sold {
        model.date_sold = trim_opt(v);
    }
    if let Some(v) = data.notes {
        model.notes = trim_opt(v);
    }
    model.updated_at = Set(now);
    LensService::update(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lens", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_one(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    LensService::delete(&db, id)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lens", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn distinct_brands(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<Vec<String>>> {
    Ok(Json(LensService::distinct_brands(&db).await?))
}

async fn distinct_systems(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<Vec<String>>> {
    Ok(Json(LensService::distinct_lens_systems(&db).await?))
}

async fn cameras_for_lens(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<i32>>> {
    Ok(Json(LensService::get_cameras_for_lens(&db, id).await?))
}
