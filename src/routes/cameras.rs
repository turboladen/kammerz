use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult, OptionExt};
use crate::patch::{double_option, now_string, trim_opt};
use crate::routes::{friendly_delete_err, friendly_err};
use crate::services::camera_service::CameraService;
use crate::services::lens_service::LensService;
use crate::validate::{require_nonempty, validate_date_opt, validate_non_negative_f64};
use crate::AppState;
use entity::camera::{self, CameraFormat, CameraType};
use entity::camera_maintenance::{self, MaintenanceType};
use entity::lens;

// --- DTOs (moved verbatim from commands/cameras.rs) ---

#[derive(Debug, Deserialize, Clone)]
pub struct CreateCameraDto {
    pub brand: String,
    pub model: String,
    pub prefix: Option<String>,
    pub format: CameraFormat,
    pub lens_mount_id: i32,
    pub default_lens_id: Option<i32>,
    pub camera_type: Option<CameraType>,
    pub serial_number: Option<String>,
    pub date_purchased: Option<String>,
    pub purchased_from: Option<String>,
    pub date_sold: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateCameraDto {
    pub brand: Option<String>,
    pub model: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub prefix: Option<Option<String>>,
    pub format: Option<CameraFormat>,
    pub lens_mount_id: Option<i32>,
    #[serde(deserialize_with = "double_option")]
    pub default_lens_id: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub camera_type: Option<Option<CameraType>>,
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

#[derive(Debug, Deserialize)]
pub struct CreateMaintenanceDto {
    pub camera_id: i32,
    pub maintenance_type: MaintenanceType,
    pub done_by: Option<String>,
    pub date_done: Option<String>,
    pub cost: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateMaintenanceDto {
    pub camera_id: Option<i32>,
    pub maintenance_type: Option<MaintenanceType>,
    #[serde(deserialize_with = "double_option")]
    pub done_by: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_done: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub cost: Option<Option<f64>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCameraWithLensDto {
    pub camera: CreateCameraDto,
    pub lens_model: Option<String>,
    pub lens_focal_length: Option<String>,
    pub lens_max_aperture: Option<String>,
}

// --- Routers ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/with-lens", post(create_with_lens))
        .route("/distinct/brands", get(distinct_brands))
        .route("/distinct/vendors", get(distinct_vendors))
        .route("/distinct/maint-providers", get(distinct_maint_providers))
        .route("/{id}", get(get_one).put(update).delete(delete_one))
        .route("/{id}/lenses", get(lenses_for_camera))
        .route(
            "/{id}/lenses/{lens_id}",
            post(link_lens).delete(unlink_lens),
        )
        .route("/{id}/maintenance", get(list_maintenance))
}

pub fn maintenance_router() -> Router<AppState> {
    Router::new().route("/", post(create_maintenance)).route(
        "/{id}",
        axum::routing::put(update_maintenance).delete(delete_maintenance),
    )
}

// --- Camera handlers ---

async fn list(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<Vec<camera::Model>>> {
    Ok(Json(CameraService::list_all(&db).await?))
}

async fn get_one(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Option<camera::Model>>> {
    Ok(Json(CameraService::get_by_id(&db, id).await?))
}

async fn create(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateCameraDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date_purchased", &data.date_purchased)?;
    validate_date_opt("date_sold", &data.date_sold)?;
    let brand = require_nonempty("brand", &data.brand)?;
    let camera_model = require_nonempty("model", &data.model)?;

    let now = now_string();
    let model = camera::ActiveModel {
        brand: Set(brand),
        model: Set(camera_model),
        prefix: trim_opt(data.prefix),
        format: Set(data.format),
        lens_mount_id: Set(data.lens_mount_id),
        default_lens_id: Set(data.default_lens_id),
        camera_type: Set(data.camera_type),
        serial_number: trim_opt(data.serial_number),
        date_purchased: trim_opt(data.date_purchased),
        purchased_from: trim_opt(data.purchased_from),
        date_sold: trim_opt(data.date_sold),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let res = CameraService::create(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("camera", e)))?;
    Ok((StatusCode::CREATED, Json(res.id)))
}

async fn update(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateCameraDto>,
) -> AppResult<StatusCode> {
    let existing = CameraService::get_by_id(&db, id)
        .await?
        .or_404("Camera", id)?;
    if let Some(v) = &data.date_purchased {
        validate_date_opt("date_purchased", v)?;
    }
    if let Some(v) = &data.date_sold {
        validate_date_opt("date_sold", v)?;
    }
    let now = now_string();
    let mut model: camera::ActiveModel = existing.into();
    if let Some(v) = data.brand {
        model.brand = Set(require_nonempty("brand", &v)?);
    }
    if let Some(v) = data.model {
        model.model = Set(require_nonempty("model", &v)?);
    }
    if let Some(v) = data.prefix {
        model.prefix = trim_opt(v);
    }
    if let Some(v) = data.format {
        model.format = Set(v);
    }
    if let Some(v) = data.lens_mount_id {
        model.lens_mount_id = Set(v);
    }
    if let Some(v) = data.default_lens_id {
        model.default_lens_id = Set(v);
    }
    if let Some(v) = data.camera_type {
        model.camera_type = Set(v);
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
    CameraService::update(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("camera", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_one(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    CameraService::delete(&db, id)
        .await
        .map_err(|e| friendly_delete_err("camera", e))?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Create camera with fixed lens (transactional) ---

async fn create_with_lens(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateCameraWithLensDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date_purchased", &data.camera.date_purchased)?;
    validate_date_opt("date_sold", &data.camera.date_sold)?;
    require_nonempty("brand", &data.camera.brand)?;
    require_nonempty("model", &data.camera.model)?;

    let now = now_string();

    let camera_id = db
        .transaction::<_, i32, DbErr>(|txn| {
            let now = now.clone();
            Box::pin(async move {
                // Pre-trim shared fields used by both camera and lens
                let brand = data.camera.brand.trim().to_string();
                let serial = data.camera.serial_number.map(|s| s.trim().to_string());
                let purchased = data.camera.date_purchased.map(|s| s.trim().to_string());
                let vendor = data.camera.purchased_from.map(|s| s.trim().to_string());

                // 1. Create camera (default_lens_id will be set after lens creation)
                let cam_model = camera::ActiveModel {
                    brand: Set(brand.clone()),
                    model: Set(data.camera.model.trim().to_string()),
                    prefix: Set(data.camera.prefix.map(|s| s.trim().to_string())),
                    format: Set(data.camera.format),
                    lens_mount_id: Set(data.camera.lens_mount_id),
                    default_lens_id: Set(None),
                    camera_type: Set(data.camera.camera_type),
                    serial_number: Set(serial.clone()),
                    date_purchased: Set(purchased.clone()),
                    purchased_from: Set(vendor.clone()),
                    date_sold: Set(data.camera.date_sold.map(|s| s.trim().to_string())),
                    notes: Set(data.camera.notes.map(|s| s.trim().to_string())),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                    ..Default::default()
                };
                let cam = CameraService::create(txn, cam_model).await?;

                // 2. Create lens (brand + mount shared with camera)
                let lens_model = lens::ActiveModel {
                    brand: Set(brand),
                    lens_mount_id: Set(data.camera.lens_mount_id),
                    lens_system: Set(None),
                    model: Set(data.lens_model.map(|s| s.trim().to_string())),
                    focal_length: Set(data.lens_focal_length.map(|s| s.trim().to_string())),
                    max_aperture: Set(data.lens_max_aperture.map(|s| s.trim().to_string())),
                    min_aperture: Set(None),
                    filter_thread_front_mm: Set(None),
                    filter_thread_rear_mm: Set(None),
                    serial_number: Set(serial),
                    date_purchased: Set(purchased),
                    purchased_from: Set(vendor),
                    date_sold: Set(None),
                    notes: Set(None),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                    ..Default::default()
                };
                let created_lens = LensService::create(txn, lens_model).await?;

                // 3. Link lens to camera
                let cam_id = cam.id;
                CameraService::link_lens(txn, cam_id, created_lens.id).await?;

                // 4. Set default_lens_id on camera
                let mut cam_update: camera::ActiveModel = cam.into();
                cam_update.default_lens_id = Set(Some(created_lens.id));
                cam_update.updated_at = Set(now);
                CameraService::update(txn, cam_update).await?;

                Ok(cam_id)
            })
        })
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("camera", e)))?;

    Ok((StatusCode::CREATED, Json(camera_id)))
}

// --- Distinct value helpers ---

async fn distinct_brands(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<Vec<String>>> {
    Ok(Json(CameraService::distinct_brands(&db).await?))
}

async fn distinct_vendors(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<Vec<String>>> {
    Ok(Json(CameraService::distinct_vendors(&db).await?))
}

async fn distinct_maint_providers(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<Vec<String>>> {
    Ok(Json(CameraService::distinct_maint_providers(&db).await?))
}

// --- Camera-Lens association handlers ---

async fn lenses_for_camera(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<i32>>> {
    Ok(Json(CameraService::get_lenses_for_camera(&db, id).await?))
}

async fn link_lens(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path((id, lens_id)): Path<(i32, i32)>,
) -> AppResult<StatusCode> {
    CameraService::link_lens(&db, id, lens_id)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("camera", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unlink_lens(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path((id, lens_id)): Path<(i32, i32)>,
) -> AppResult<StatusCode> {
    CameraService::unlink_lens(&db, id, lens_id)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("camera", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Maintenance handlers ---

async fn list_maintenance(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<camera_maintenance::Model>>> {
    Ok(Json(CameraService::list_maintenance(&db, id).await?))
}

async fn create_maintenance(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateMaintenanceDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date_done", &data.date_done)?;
    validate_non_negative_f64("cost", data.cost)?;

    let now = now_string();
    let model = camera_maintenance::ActiveModel {
        camera_id: Set(data.camera_id),
        maintenance_type: Set(data.maintenance_type),
        done_by: trim_opt(data.done_by),
        date_done: trim_opt(data.date_done),
        cost: Set(data.cost),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let res = CameraService::create_maintenance(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("maintenance record", e)))?;
    Ok((StatusCode::CREATED, Json(res.id)))
}

async fn update_maintenance(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateMaintenanceDto>,
) -> AppResult<StatusCode> {
    let existing = camera_maintenance::Entity::find_by_id(id)
        .one(&db)
        .await?
        .or_404("Maintenance record", id)?;
    if let Some(v) = &data.date_done {
        validate_date_opt("date_done", v)?;
    }
    if let Some(v) = data.cost {
        validate_non_negative_f64("cost", v)?;
    }
    let now = now_string();
    let mut model: camera_maintenance::ActiveModel = existing.into();
    if let Some(v) = data.camera_id {
        model.camera_id = Set(v);
    }
    if let Some(v) = data.maintenance_type {
        model.maintenance_type = Set(v);
    }
    if let Some(v) = data.done_by {
        model.done_by = trim_opt(v);
    }
    if let Some(v) = data.date_done {
        model.date_done = trim_opt(v);
    }
    if let Some(v) = data.cost {
        model.cost = Set(v);
    }
    if let Some(v) = data.notes {
        model.notes = trim_opt(v);
    }
    model.updated_at = Set(now);
    CameraService::update_maintenance(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("maintenance record", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_maintenance(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    CameraService::delete_maintenance(&db, id)
        .await
        .map_err(|e| friendly_delete_err("maintenance record", e))?;
    Ok(StatusCode::NO_CONTENT)
}
