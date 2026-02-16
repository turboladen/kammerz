use sea_orm::{EntityTrait, Set};
use serde::Deserialize;
use tauri::State;

use crate::entities::shot;
use crate::services::shot_service::ShotService;
use crate::AppState;

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct CreateShotDto {
    pub roll_id: i32,
    pub frame_number: String,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub date: Option<String>,
    pub date_fuzzy: Option<String>,
    pub location: Option<String>,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    pub notes: Option<String>,
    pub lens_ids: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateShotDto {
    pub frame_number: Option<String>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub date: Option<String>,
    pub date_fuzzy: Option<String>,
    pub location: Option<String>,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
    pub notes: Option<String>,
    pub lens_ids: Option<Vec<i32>>,
}

// --- Commands ---

#[tauri::command]
pub async fn list_shots_for_roll(
    state: State<'_, AppState>,
    roll_id: i32,
) -> Result<Vec<shot::Model>, String> {
    ShotService::list_for_roll(&state.db, roll_id)
        .await
        .map_err(|e| {
            log::error!("Failed to list shots for roll {roll_id}: {e}");
            format!("Could not list shots: {e}")
        })
}

#[tauri::command]
pub async fn get_shot(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<shot::Model>, String> {
    ShotService::get_by_id(&state.db, id).await.map_err(|e| {
        log::error!("Failed to get shot {id}: {e}");
        format!("Could not get shot: {e}")
    })
}

#[tauri::command]
pub async fn create_shot(
    state: State<'_, AppState>,
    data: CreateShotDto,
) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = shot::ActiveModel {
        roll_id: Set(data.roll_id),
        frame_number: Set(data.frame_number),
        aperture: Set(data.aperture),
        shutter_speed: Set(data.shutter_speed),
        date: Set(data.date),
        date_fuzzy: Set(data.date_fuzzy),
        location: Set(data.location),
        gps_lat: Set(data.gps_lat),
        gps_lon: Set(data.gps_lon),
        notes: Set(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let result = ShotService::create(&state.db, model).await.map_err(|e| {
        log::error!("Failed to create shot: {e}");
        format!("Could not create shot: {e}")
    })?;

    if let Some(lens_ids) = data.lens_ids {
        if !lens_ids.is_empty() {
            ShotService::set_lenses_for_shot(&state.db, result.id, lens_ids)
                .await
                .map_err(|e| {
                    log::error!("Failed to set lenses for shot {}: {e}", result.id);
                    format!("Could not set lenses: {e}")
                })?;
        }
    }

    Ok(result.id)
}

#[tauri::command]
pub async fn update_shot(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateShotDto,
) -> Result<(), String> {
    let existing = shot::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .map_err(|e| format!("Could not find shot: {e}"))?
        .ok_or_else(|| format!("Shot {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: shot::ActiveModel = existing.into();

    if let Some(v) = data.frame_number { model.frame_number = Set(v); }
    if data.aperture.is_some() { model.aperture = Set(data.aperture); }
    if data.shutter_speed.is_some() { model.shutter_speed = Set(data.shutter_speed); }
    if data.date.is_some() { model.date = Set(data.date); }
    if data.date_fuzzy.is_some() { model.date_fuzzy = Set(data.date_fuzzy); }
    if data.location.is_some() { model.location = Set(data.location); }
    if data.gps_lat.is_some() { model.gps_lat = Set(data.gps_lat); }
    if data.gps_lon.is_some() { model.gps_lon = Set(data.gps_lon); }
    if data.notes.is_some() { model.notes = Set(data.notes); }
    model.updated_at = Set(now);

    ShotService::update(&state.db, model).await.map_err(|e| {
        log::error!("Failed to update shot {id}: {e}");
        format!("Could not update shot: {e}")
    })?;

    if let Some(lens_ids) = data.lens_ids {
        ShotService::set_lenses_for_shot(&state.db, id, lens_ids)
            .await
            .map_err(|e| {
                log::error!("Failed to set lenses for shot {id}: {e}");
                format!("Could not set lenses: {e}")
            })?;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_shot(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    ShotService::delete(&state.db, id).await.map_err(|e| {
        log::error!("Failed to delete shot {id}: {e}");
        format!("Could not delete shot: {e}")
    })
}

#[tauri::command]
pub async fn get_lenses_for_shot(
    state: State<'_, AppState>,
    shot_id: i32,
) -> Result<Vec<i32>, String> {
    ShotService::get_lenses_for_shot(&state.db, shot_id)
        .await
        .map_err(|e| {
            log::error!("Failed to get lenses for shot {shot_id}: {e}");
            format!("Could not get lenses: {e}")
        })
}

#[tauri::command]
pub async fn suggest_next_frame(
    state: State<'_, AppState>,
    roll_id: i32,
) -> Result<String, String> {
    ShotService::suggest_next_frame(&state.db, roll_id)
        .await
        .map_err(|e| {
            log::error!("Failed to suggest next frame for roll {roll_id}: {e}");
            format!("Could not suggest frame: {e}")
        })
}

#[tauri::command]
pub async fn count_shots_for_roll(
    state: State<'_, AppState>,
    roll_id: i32,
) -> Result<u64, String> {
    ShotService::count_for_roll(&state.db, roll_id)
        .await
        .map_err(|e| {
            log::error!("Failed to count shots for roll {roll_id}: {e}");
            format!("Could not count shots: {e}")
        })
}
