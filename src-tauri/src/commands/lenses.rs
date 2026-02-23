use sea_orm::Set;
use serde::Deserialize;
use tauri::State;

use crate::entities::lens;
use crate::patch::{double_option, trim, trim_opt};
use crate::services::lens_service::LensService;
use crate::AppState;

// --- DTOs ---

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

// --- Commands ---

#[tauri::command]
pub async fn list_lenses(state: State<'_, AppState>) -> Result<Vec<lens::Model>, String> {
    LensService::list_all(&state.db).await.map_err(|e| {
        log::error!("Failed to list lenses: {e}");
        format!("Could not list lenses: {e}")
    })
}

#[tauri::command]
pub async fn get_lens(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<lens::Model>, String> {
    LensService::get_by_id(&state.db, id).await.map_err(|e| {
        log::error!("Failed to get lens {id}: {e}");
        format!("Could not get lens: {e}")
    })
}

#[tauri::command]
pub async fn create_lens(state: State<'_, AppState>, data: CreateLensDto) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
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
    let result = LensService::create(&state.db, model).await.map_err(|e| {
        log::error!("Failed to create lens: {e}");
        format!("Could not create lens: {e}")
    })?;
    Ok(result.id)
}

#[tauri::command]
pub async fn update_lens(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateLensDto,
) -> Result<(), String> {
    let existing = LensService::get_by_id(&state.db, id)
        .await
        .map_err(|e| format!("Could not find lens: {e}"))?
        .ok_or_else(|| format!("Lens {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: lens::ActiveModel = existing.into();

    if let Some(v) = data.brand { model.brand = trim(v); }
    if let Some(v) = data.lens_mount_id { model.lens_mount_id = Set(v); }
    if let Some(v) = data.lens_system { model.lens_system = trim_opt(v); }
    if let Some(v) = data.model { model.model = trim_opt(v); }
    if let Some(v) = data.focal_length { model.focal_length = trim_opt(v); }
    if let Some(v) = data.max_aperture { model.max_aperture = trim_opt(v); }
    if let Some(v) = data.min_aperture { model.min_aperture = trim_opt(v); }
    if let Some(v) = data.filter_thread_front_mm { model.filter_thread_front_mm = Set(v); }
    if let Some(v) = data.filter_thread_rear_mm { model.filter_thread_rear_mm = Set(v); }
    if let Some(v) = data.serial_number { model.serial_number = trim_opt(v); }
    if let Some(v) = data.date_purchased { model.date_purchased = trim_opt(v); }
    if let Some(v) = data.purchased_from { model.purchased_from = trim_opt(v); }
    if let Some(v) = data.date_sold { model.date_sold = trim_opt(v); }
    if let Some(v) = data.notes { model.notes = trim_opt(v); }
    model.updated_at = Set(now);

    LensService::update(&state.db, model).await.map_err(|e| {
        log::error!("Failed to update lens {id}: {e}");
        format!("Could not update lens: {e}")
    })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_lens(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    LensService::delete(&state.db, id).await.map_err(|e| {
        log::error!("Failed to delete lens {id}: {e}");
        format!("Could not delete lens: {e}")
    })
}

// --- Distinct value helpers ---

#[tauri::command]
pub async fn list_distinct_lens_brands(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    LensService::distinct_brands(&state.db)
        .await
        .map_err(|e| format!("Could not list lens brands: {e}"))
}

#[tauri::command]
pub async fn list_distinct_lens_systems(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    LensService::distinct_lens_systems(&state.db)
        .await
        .map_err(|e| format!("Could not list lens systems: {e}"))
}

// --- Camera association (reverse lookup) ---

#[tauri::command]
pub async fn get_cameras_for_lens(
    state: State<'_, AppState>,
    lens_id: i32,
) -> Result<Vec<i32>, String> {
    LensService::get_cameras_for_lens(&state.db, lens_id)
        .await
        .map_err(|e| {
            log::error!("Failed to get cameras for lens {lens_id}: {e}");
            format!("Could not get cameras for lens: {e}")
        })
}
