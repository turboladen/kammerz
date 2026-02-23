use sea_orm::{DbErr, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;
use tauri::State;

use crate::entities::roll::{self, RollStatus};
use crate::entities::shot;
use crate::patch::{double_option, trim, trim_opt};
use crate::services::roll_service::RollService;
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

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateShotDto {
    pub frame_number: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub aperture: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub shutter_speed: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_fuzzy: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub location: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub gps_lat: Option<Option<f64>>,
    #[serde(deserialize_with = "double_option")]
    pub gps_lon: Option<Option<f64>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
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

    let result_id = state
        .db
        .transaction::<_, i32, DbErr>(|txn| {
            Box::pin(async move {
                let model = shot::ActiveModel {
                    roll_id: Set(data.roll_id),
                    frame_number: trim(data.frame_number),
                    aperture: trim_opt(data.aperture),
                    shutter_speed: trim_opt(data.shutter_speed),
                    date: trim_opt(data.date),
                    date_fuzzy: trim_opt(data.date_fuzzy),
                    location: trim_opt(data.location),
                    gps_lat: Set(data.gps_lat),
                    gps_lon: Set(data.gps_lon),
                    notes: trim_opt(data.notes),
                    created_at: Set(now.clone()),
                    updated_at: Set(now),
                    ..Default::default()
                };
                let result = ShotService::create(txn, model).await?;

                if let Some(lens_ids) = data.lens_ids {
                    if !lens_ids.is_empty() {
                        ShotService::set_lenses_for_shot(txn, result.id, lens_ids).await?;
                    }
                }

                // Auto-transition: loaded → shooting when first shot is added
                let roll_record = roll::Entity::find_by_id(data.roll_id)
                    .one(txn)
                    .await?
                    .ok_or_else(|| DbErr::Custom(format!("Roll {} not found", data.roll_id)))?;

                if roll_record.status == RollStatus::Loaded {
                    let now_ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    let mut roll_model: roll::ActiveModel = roll_record.into();
                    roll_model.status = Set(RollStatus::Shooting);
                    roll_model.updated_at = Set(now_ts);
                    RollService::update(txn, roll_model).await?;
                }

                Ok(result.id)
            })
        })
        .await
        .map_err(|e| {
            log::error!("Failed to create shot: {e}");
            format!("Could not create shot: {e}")
        })?;

    Ok(result_id)
}

#[tauri::command]
pub async fn update_shot(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateShotDto,
) -> Result<(), String> {
    let existing = ShotService::get_by_id(&state.db, id)
        .await
        .map_err(|e| format!("Could not find shot: {e}"))?
        .ok_or_else(|| format!("Shot {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    state
        .db
        .transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                let mut model: shot::ActiveModel = existing.into();

                if let Some(v) = data.frame_number { model.frame_number = trim(v); }
                if let Some(v) = data.aperture { model.aperture = trim_opt(v); }
                if let Some(v) = data.shutter_speed { model.shutter_speed = trim_opt(v); }
                if let Some(v) = data.date { model.date = trim_opt(v); }
                if let Some(v) = data.date_fuzzy { model.date_fuzzy = trim_opt(v); }
                if let Some(v) = data.location { model.location = trim_opt(v); }
                if let Some(v) = data.gps_lat { model.gps_lat = Set(v); }
                if let Some(v) = data.gps_lon { model.gps_lon = Set(v); }
                if let Some(v) = data.notes { model.notes = trim_opt(v); }
                model.updated_at = Set(now);

                ShotService::update(txn, model).await?;

                if let Some(lens_ids) = data.lens_ids {
                    ShotService::set_lenses_for_shot(txn, id, lens_ids).await?;
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| {
            log::error!("Failed to update shot {id}: {e}");
            format!("Could not update shot: {e}")
        })?;

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
pub async fn get_lenses_for_roll_shots(
    state: State<'_, AppState>,
    roll_id: i32,
) -> Result<Vec<(i32, i32)>, String> {
    ShotService::get_lenses_for_roll_shots(&state.db, roll_id)
        .await
        .map_err(|e| {
            log::error!("Failed to get lenses for roll shots {roll_id}: {e}");
            format!("Could not get shot lenses: {e}")
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
