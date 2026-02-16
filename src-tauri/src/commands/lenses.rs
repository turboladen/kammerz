use sea_orm::Set;
use serde::Deserialize;
use tauri::State;

use crate::entities::lens;
use crate::services::lens_service::LensService;
use crate::AppState;

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct CreateLensDto {
    pub brand: String,
    pub lens_system: Option<String>,
    pub name_on_lens: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct UpdateLensDto {
    pub brand: Option<String>,
    pub lens_system: Option<String>,
    pub name_on_lens: Option<String>,
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
        brand: Set(data.brand),
        lens_system: Set(data.lens_system),
        name_on_lens: Set(data.name_on_lens),
        focal_length: Set(data.focal_length),
        max_aperture: Set(data.max_aperture),
        min_aperture: Set(data.min_aperture),
        filter_thread_front_mm: Set(data.filter_thread_front_mm),
        filter_thread_rear_mm: Set(data.filter_thread_rear_mm),
        serial_number: Set(data.serial_number),
        date_purchased: Set(data.date_purchased),
        purchased_from: Set(data.purchased_from),
        date_sold: Set(data.date_sold),
        notes: Set(data.notes),
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

    if let Some(v) = data.brand { model.brand = Set(v); }
    if data.lens_system.is_some() { model.lens_system = Set(data.lens_system); }
    if data.name_on_lens.is_some() { model.name_on_lens = Set(data.name_on_lens); }
    if data.focal_length.is_some() { model.focal_length = Set(data.focal_length); }
    if data.max_aperture.is_some() { model.max_aperture = Set(data.max_aperture); }
    if data.min_aperture.is_some() { model.min_aperture = Set(data.min_aperture); }
    if data.filter_thread_front_mm.is_some() { model.filter_thread_front_mm = Set(data.filter_thread_front_mm); }
    if data.filter_thread_rear_mm.is_some() { model.filter_thread_rear_mm = Set(data.filter_thread_rear_mm); }
    if data.serial_number.is_some() { model.serial_number = Set(data.serial_number); }
    if data.date_purchased.is_some() { model.date_purchased = Set(data.date_purchased); }
    if data.purchased_from.is_some() { model.purchased_from = Set(data.purchased_from); }
    if data.date_sold.is_some() { model.date_sold = Set(data.date_sold); }
    if data.notes.is_some() { model.notes = Set(data.notes); }
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
