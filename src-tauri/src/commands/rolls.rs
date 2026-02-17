use sea_orm::{EntityTrait, Set};
use serde::Deserialize;
use tauri::State;

use crate::entities::roll;
use crate::services::roll_service::{RollService, RollWithDetails};
use crate::AppState;

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct CreateRollDto {
    pub roll_id: String,
    pub camera_id: Option<i32>,
    pub film_stock_id: Option<i32>,
    pub lens_id: Option<i32>,
    pub status: String,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub date_fuzzy: Option<String>,
    pub push_pull: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRollDto {
    pub roll_id: Option<String>,
    pub camera_id: Option<i32>,
    pub film_stock_id: Option<i32>,
    pub lens_id: Option<i32>,
    pub status: Option<String>,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub date_fuzzy: Option<String>,
    pub push_pull: Option<String>,
    pub notes: Option<String>,
}

// --- Commands ---

#[tauri::command]
pub async fn list_rolls(state: State<'_, AppState>) -> Result<Vec<RollWithDetails>, String> {
    RollService::list_all_with_details(&state.db)
        .await
        .map_err(|e| {
            log::error!("Failed to list rolls: {e}");
            format!("Could not list rolls: {e}")
        })
}

#[tauri::command]
pub async fn get_roll(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<RollWithDetails>, String> {
    RollService::get_with_details(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to get roll {id}: {e}");
            format!("Could not get roll: {e}")
        })
}

#[tauri::command]
pub async fn create_roll(state: State<'_, AppState>, data: CreateRollDto) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = roll::ActiveModel {
        roll_id: Set(data.roll_id),
        camera_id: Set(data.camera_id),
        film_stock_id: Set(data.film_stock_id),
        lens_id: Set(data.lens_id),
        status: Set(data.status),
        frame_count: Set(data.frame_count),
        date_loaded: Set(data.date_loaded),
        date_finished: Set(data.date_finished),
        date_fuzzy: Set(data.date_fuzzy),
        push_pull: Set(data.push_pull),
        notes: Set(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let result = RollService::create(&state.db, model).await.map_err(|e| {
        log::error!("Failed to create roll: {e}");
        format!("Could not create roll: {e}")
    })?;
    Ok(result.id)
}

#[tauri::command]
pub async fn update_roll(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateRollDto,
) -> Result<(), String> {
    let existing = roll::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|e| format!("Could not find roll: {e}"))?
        .ok_or_else(|| format!("Roll {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: roll::ActiveModel = existing.into();

    if let Some(v) = data.roll_id { model.roll_id = Set(v); }
    model.camera_id = Set(data.camera_id);
    model.film_stock_id = Set(data.film_stock_id);
    model.lens_id = Set(data.lens_id);
    if let Some(v) = data.status { model.status = Set(v); }
    if data.frame_count.is_some() { model.frame_count = Set(data.frame_count); }
    if data.date_loaded.is_some() { model.date_loaded = Set(data.date_loaded); }
    if data.date_finished.is_some() { model.date_finished = Set(data.date_finished); }
    if data.date_fuzzy.is_some() { model.date_fuzzy = Set(data.date_fuzzy); }
    if data.push_pull.is_some() { model.push_pull = Set(data.push_pull); }
    if data.notes.is_some() { model.notes = Set(data.notes); }
    model.updated_at = Set(now);

    RollService::update(&state.db, model).await.map_err(|e| {
        log::error!("Failed to update roll {id}: {e}");
        format!("Could not update roll: {e}")
    })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_roll(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    RollService::delete(&state.db, id).await.map_err(|e| {
        log::error!("Failed to delete roll {id}: {e}");
        format!("Could not delete roll: {e}")
    })
}

#[tauri::command]
pub async fn list_rolls_for_camera(
    state: State<'_, AppState>,
    camera_id: i32,
) -> Result<Vec<RollWithDetails>, String> {
    RollService::list_for_camera(&state.db, camera_id)
        .await
        .map_err(|e| {
            log::error!("Failed to list rolls for camera {camera_id}: {e}");
            format!("Could not list rolls: {e}")
        })
}

#[tauri::command]
pub async fn suggest_roll_id(state: State<'_, AppState>) -> Result<String, String> {
    RollService::suggest_id(&state.db).await.map_err(|e| {
        log::error!("Failed to suggest roll ID: {e}");
        format!("Could not suggest roll ID: {e}")
    })
}
