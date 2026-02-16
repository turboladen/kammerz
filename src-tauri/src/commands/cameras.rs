use sea_orm::{EntityTrait, Set};
use serde::Deserialize;
use tauri::State;

use crate::entities::{camera, camera_maintenance};
use crate::services::camera_service::CameraService;
use crate::AppState;

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct CreateCameraDto {
    pub brand: String,
    pub model: String,
    pub prefix: Option<String>,
    pub format: String,
    pub camera_type: Option<String>,
    pub serial_number: Option<String>,
    pub date_purchased: Option<String>,
    pub purchased_from: Option<String>,
    pub date_sold: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCameraDto {
    pub brand: Option<String>,
    pub model: Option<String>,
    pub prefix: Option<String>,
    pub format: Option<String>,
    pub camera_type: Option<String>,
    pub serial_number: Option<String>,
    pub date_purchased: Option<String>,
    pub purchased_from: Option<String>,
    pub date_sold: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMaintenanceDto {
    pub camera_id: i32,
    pub maintenance_type: String,
    pub done_by: Option<String>,
    pub date_done: Option<String>,
    pub cost: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMaintenanceDto {
    pub camera_id: Option<i32>,
    pub maintenance_type: Option<String>,
    pub done_by: Option<String>,
    pub date_done: Option<String>,
    pub cost: Option<f64>,
    pub notes: Option<String>,
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
        brand: Set(data.brand),
        model: Set(data.model),
        prefix: Set(data.prefix),
        format: Set(data.format),
        camera_type: Set(data.camera_type),
        serial_number: Set(data.serial_number),
        date_purchased: Set(data.date_purchased),
        purchased_from: Set(data.purchased_from),
        date_sold: Set(data.date_sold),
        notes: Set(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let result = CameraService::create(&state.db, model).await.map_err(|e| {
        log::error!("Failed to create camera: {e}");
        format!("Could not create camera: {e}")
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

    if let Some(v) = data.brand { model.brand = Set(v); }
    if let Some(v) = data.model { model.model = Set(v); }
    if data.prefix.is_some() { model.prefix = Set(data.prefix); }
    if let Some(v) = data.format { model.format = Set(v); }
    if data.camera_type.is_some() { model.camera_type = Set(data.camera_type); }
    if data.serial_number.is_some() { model.serial_number = Set(data.serial_number); }
    if data.date_purchased.is_some() { model.date_purchased = Set(data.date_purchased); }
    if data.purchased_from.is_some() { model.purchased_from = Set(data.purchased_from); }
    if data.date_sold.is_some() { model.date_sold = Set(data.date_sold); }
    if data.notes.is_some() { model.notes = Set(data.notes); }
    model.updated_at = Set(now);

    CameraService::update(&state.db, model).await.map_err(|e| {
        log::error!("Failed to update camera {id}: {e}");
        format!("Could not update camera: {e}")
    })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_camera(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    CameraService::delete(&state.db, id).await.map_err(|e| {
        log::error!("Failed to delete camera {id}: {e}");
        format!("Could not delete camera: {e}")
    })
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
        done_by: Set(data.done_by),
        date_done: Set(data.date_done),
        cost: Set(data.cost),
        notes: Set(data.notes),
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
    if data.done_by.is_some() { model.done_by = Set(data.done_by); }
    if data.date_done.is_some() { model.date_done = Set(data.date_done); }
    if data.cost.is_some() { model.cost = Set(data.cost); }
    if data.notes.is_some() { model.notes = Set(data.notes); }
    model.updated_at = Set(now);

    CameraService::update_maintenance(&state.db, model)
        .await
        .map_err(|e| {
            log::error!("Failed to update maintenance {id}: {e}");
            format!("Could not update maintenance: {e}")
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
