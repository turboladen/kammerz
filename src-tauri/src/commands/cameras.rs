use sea_orm::{DbErr, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;
use tauri::State;

use crate::entities::camera::{self, CameraFormat, CameraType};
use crate::entities::camera_maintenance::{self, MaintenanceType};
use crate::entities::lens;
use crate::patch::{double_option, trim, trim_opt};
use crate::services::camera_service::CameraService;
use crate::services::lens_service::LensService;
use crate::AppState;

// --- DTOs ---

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

// --- Camera commands ---

#[tauri::command]
pub async fn list_cameras(state: State<'_, AppState>) -> Result<Vec<camera::Model>, String> {
    CameraService::list_all(&state.db).await.map_err(|e| {
        log::error!("Failed to list cameras: {e}");
        format!("Could not list cameras: {e}")
    })
}

#[tauri::command]
pub async fn get_camera(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<camera::Model>, String> {
    CameraService::get_by_id(&state.db, id).await.map_err(|e| {
        log::error!("Failed to get camera {id}: {e}");
        format!("Could not get camera: {e}")
    })
}

#[tauri::command]
pub async fn create_camera(
    state: State<'_, AppState>,
    data: CreateCameraDto,
) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = camera::ActiveModel {
        brand: trim(data.brand),
        model: trim(data.model),
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
    let result = CameraService::create(&state.db, model).await.map_err(|e| {
        log::error!("Failed to create camera: {e}");
        super::friendly_err("camera", e)
    })?;
    Ok(result.id)
}

#[tauri::command]
pub async fn update_camera(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateCameraDto,
) -> Result<(), String> {
    let existing = CameraService::get_by_id(&state.db, id)
        .await
        .map_err(|e| format!("Could not find camera: {e}"))?
        .ok_or_else(|| format!("Camera {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: camera::ActiveModel = existing.into();

    if let Some(v) = data.brand { model.brand = trim(v); }
    if let Some(v) = data.model { model.model = trim(v); }
    if let Some(v) = data.prefix { model.prefix = trim_opt(v); }
    if let Some(v) = data.format { model.format = Set(v); }
    if let Some(v) = data.lens_mount_id { model.lens_mount_id = Set(v); }
    if let Some(v) = data.default_lens_id { model.default_lens_id = Set(v); }
    if let Some(v) = data.camera_type { model.camera_type = Set(v); }
    if let Some(v) = data.serial_number { model.serial_number = trim_opt(v); }
    if let Some(v) = data.date_purchased { model.date_purchased = trim_opt(v); }
    if let Some(v) = data.purchased_from { model.purchased_from = trim_opt(v); }
    if let Some(v) = data.date_sold { model.date_sold = trim_opt(v); }
    if let Some(v) = data.notes { model.notes = trim_opt(v); }
    model.updated_at = Set(now);

    CameraService::update(&state.db, model).await.map_err(|e| {
        log::error!("Failed to update camera {id}: {e}");
        super::friendly_err("camera", e)
    })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_camera(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    CameraService::delete(&state.db, id).await.map_err(|e| {
        log::error!("Failed to delete camera {id}: {e}");
        super::friendly_err("camera", e)
    })
}

// --- Create camera with fixed lens (transactional) ---

#[derive(Debug, Deserialize)]
pub struct CreateCameraWithLensDto {
    pub camera: CreateCameraDto,
    pub lens_model: Option<String>,
    pub lens_focal_length: Option<String>,
    pub lens_max_aperture: Option<String>,
}

#[tauri::command]
pub async fn create_camera_with_lens(
    state: State<'_, AppState>,
    data: CreateCameraWithLensDto,
) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let camera_id = state
        .db
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
        .map_err(|e| {
            log::error!("Failed to create camera with lens: {e}");
            super::friendly_err("camera", e)
        })?;

    Ok(camera_id)
}

// --- Maintenance commands ---

#[tauri::command]
pub async fn list_maintenance(
    state: State<'_, AppState>,
    camera_id: i32,
) -> Result<Vec<camera_maintenance::Model>, String> {
    CameraService::list_maintenance(&state.db, camera_id)
        .await
        .map_err(|e| {
            log::error!("Failed to list maintenance for camera {camera_id}: {e}");
            format!("Could not list maintenance: {e}")
        })
}

#[tauri::command]
pub async fn create_maintenance(
    state: State<'_, AppState>,
    data: CreateMaintenanceDto,
) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
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
    let result = CameraService::create_maintenance(&state.db, model)
        .await
        .map_err(|e| {
            log::error!("Failed to create maintenance: {e}");
            format!("Could not create maintenance: {e}")
        })?;
    Ok(result.id)
}

#[tauri::command]
pub async fn update_maintenance(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateMaintenanceDto,
) -> Result<(), String> {
    let existing = camera_maintenance::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|e| format!("Could not find maintenance record: {e}"))?
        .ok_or_else(|| format!("Maintenance record {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: camera_maintenance::ActiveModel = existing.into();

    if let Some(v) = data.camera_id { model.camera_id = Set(v); }
    if let Some(v) = data.maintenance_type { model.maintenance_type = Set(v); }
    if let Some(v) = data.done_by { model.done_by = trim_opt(v); }
    if let Some(v) = data.date_done { model.date_done = trim_opt(v); }
    if let Some(v) = data.cost { model.cost = Set(v); }
    if let Some(v) = data.notes { model.notes = trim_opt(v); }
    model.updated_at = Set(now);

    CameraService::update_maintenance(&state.db, model)
        .await
        .map_err(|e| {
            log::error!("Failed to update maintenance {id}: {e}");
            super::friendly_err("maintenance record", e)
        })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_maintenance(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    CameraService::delete_maintenance(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to delete maintenance {id}: {e}");
            format!("Could not delete maintenance: {e}")
        })
}

// --- Distinct value helpers ---

#[tauri::command]
pub async fn list_distinct_camera_brands(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    CameraService::distinct_brands(&state.db)
        .await
        .map_err(|e| format!("Could not list brands: {e}"))
}

#[tauri::command]
pub async fn list_distinct_vendors(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    CameraService::distinct_vendors(&state.db)
        .await
        .map_err(|e| format!("Could not list vendors: {e}"))
}

#[tauri::command]
pub async fn list_distinct_maint_providers(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    CameraService::distinct_maint_providers(&state.db)
        .await
        .map_err(|e| format!("Could not list maintenance providers: {e}"))
}

// --- Camera-Lens association commands ---

#[tauri::command]
pub async fn get_lenses_for_camera(
    state: State<'_, AppState>,
    camera_id: i32,
) -> Result<Vec<i32>, String> {
    CameraService::get_lenses_for_camera(&state.db, camera_id)
        .await
        .map_err(|e| {
            log::error!("Failed to get lenses for camera {camera_id}: {e}");
            format!("Could not get lenses for camera: {e}")
        })
}

#[tauri::command]
pub async fn link_lens_to_camera(
    state: State<'_, AppState>,
    camera_id: i32,
    lens_id: i32,
) -> Result<(), String> {
    CameraService::link_lens(&state.db, camera_id, lens_id)
        .await
        .map_err(|e| {
            log::error!("Failed to link lens {lens_id} to camera {camera_id}: {e}");
            format!("Could not link lens to camera: {e}")
        })
}

#[tauri::command]
pub async fn unlink_lens_from_camera(
    state: State<'_, AppState>,
    camera_id: i32,
    lens_id: i32,
) -> Result<(), String> {
    CameraService::unlink_lens(&state.db, camera_id, lens_id)
        .await
        .map_err(|e| {
            log::error!("Failed to unlink lens {lens_id} from camera {camera_id}: {e}");
            format!("Could not unlink lens from camera: {e}")
        })
}
