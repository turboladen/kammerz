use sea_orm::Set;
use serde::Deserialize;
use tauri::State;

use crate::entities::lab;
use crate::patch::{double_option, trim, trim_opt};
use crate::services::lab_service::LabService;
use crate::AppState;

// --- DTOs ---

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

// --- Commands ---

#[tauri::command]
pub async fn list_labs(state: State<'_, AppState>) -> Result<Vec<lab::Model>, String> {
    LabService::list_all(&state.db).await.map_err(|e| {
        log::error!("Failed to list labs: {e}");
        format!("Could not list labs: {e}")
    })
}

#[tauri::command]
pub async fn get_lab(state: State<'_, AppState>, id: i32) -> Result<Option<lab::Model>, String> {
    LabService::get_by_id(&state.db, id).await.map_err(|e| {
        log::error!("Failed to get lab {id}: {e}");
        format!("Could not get lab: {e}")
    })
}

#[tauri::command]
pub async fn create_lab(state: State<'_, AppState>, data: CreateLabDto) -> Result<i32, String> {
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
    let result = LabService::create(&state.db, model).await.map_err(|e| {
        log::error!("Failed to create lab: {e}");
        format!("Could not create lab: {e}")
    })?;
    Ok(result.id)
}

#[tauri::command]
pub async fn update_lab(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateLabDto,
) -> Result<(), String> {
    let existing = LabService::get_by_id(&state.db, id)
        .await
        .map_err(|e| format!("Could not find lab: {e}"))?
        .ok_or_else(|| format!("Lab {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: lab::ActiveModel = existing.into();

    if let Some(v) = data.name { model.name = trim(v); }
    if let Some(v) = data.location { model.location = trim_opt(v); }
    if let Some(v) = data.website { model.website = trim_opt(v); }
    if let Some(v) = data.notes { model.notes = trim_opt(v); }
    model.updated_at = Set(now);

    LabService::update(&state.db, model).await.map_err(|e| {
        log::error!("Failed to update lab {id}: {e}");
        format!("Could not update lab: {e}")
    })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_lab(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    LabService::delete(&state.db, id).await.map_err(|e| {
        log::error!("Failed to delete lab {id}: {e}");
        format!("Could not delete lab: {e}")
    })
}
