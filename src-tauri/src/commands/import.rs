use sea_orm::{ActiveModelTrait, DbErr, Set, TransactionTrait};
use serde::Deserialize;
use tauri::State;

use crate::entities::{roll, shot};
use crate::services::import_service::{ImportService, ModelInfo, ParsedRoll};
use crate::services::settings_service::SettingsService;
use crate::services::shot_service::ShotService;
use crate::AppState;

const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct ImportRollDto {
    pub roll_id: String,
    pub camera_id: Option<i32>,
    pub film_stock_id: Option<i32>,
    pub lens_id: Option<i32>,
    pub status: String,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub date_fuzzy: Option<String>,
    pub notes: Option<String>,
    pub shots: Vec<ImportShotDto>,
}

#[derive(Debug, Deserialize)]
pub struct ImportShotDto {
    pub frame_number: String,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub date: Option<String>,
    pub date_fuzzy: Option<String>,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub lens_ids: Option<Vec<i32>>,
}

// --- Commands ---

#[tauri::command]
pub async fn list_models(
    state: State<'_, AppState>,
) -> Result<Vec<ModelInfo>, String> {
    let api_key = SettingsService::get_setting(&state.db, "claude_api_key")
        .await
        .map_err(|e| format!("Could not read API key: {e}"))?
        .ok_or_else(|| {
            "No API key configured. Please set your Claude API key first.".to_string()
        })?;

    ImportService::list_models(&api_key).await
}

#[tauri::command]
pub async fn parse_note(
    state: State<'_, AppState>,
    note_text: String,
    model: Option<String>,
) -> Result<ParsedRoll, String> {
    let api_key = SettingsService::get_setting(&state.db, "claude_api_key")
        .await
        .map_err(|e| format!("Could not read API key: {e}"))?
        .ok_or_else(|| {
            "No API key configured. Please set your Claude API key first.".to_string()
        })?;

    let model = match model {
        Some(m) if !m.is_empty() => m,
        _ => {
            SettingsService::get_setting(&state.db, "claude_model")
                .await
                .map_err(|e| format!("Could not read model setting: {e}"))?
                .unwrap_or_else(|| DEFAULT_MODEL.to_string())
        }
    };

    ImportService::parse_note(&api_key, &model, &note_text).await
}

#[tauri::command]
pub async fn import_parsed_roll(
    state: State<'_, AppState>,
    data: ImportRollDto,
) -> Result<i32, String> {
    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let roll_id = state
        .db
        .transaction::<_, i32, DbErr>(|txn| {
            let now = now.clone();
            Box::pin(async move {
                // Create the roll
                let roll_model = roll::ActiveModel {
                    roll_id: Set(data.roll_id),
                    camera_id: Set(data.camera_id),
                    film_stock_id: Set(data.film_stock_id),
                    lens_id: Set(data.lens_id),
                    status: Set(data.status),
                    frame_count: Set(data.frame_count),
                    date_loaded: Set(data.date_loaded),
                    date_finished: Set(data.date_finished),
                    date_fuzzy: Set(data.date_fuzzy),
                    notes: Set(data.notes),
                    created_at: Set(now.clone()),
                    updated_at: Set(now.clone()),
                    ..Default::default()
                };
                let roll_result = roll_model.insert(txn).await?;
                let new_roll_id = roll_result.id;

                // Create shots
                for shot_dto in data.shots {
                    let shot_model = shot::ActiveModel {
                        roll_id: Set(new_roll_id),
                        frame_number: Set(shot_dto.frame_number),
                        aperture: Set(shot_dto.aperture),
                        shutter_speed: Set(shot_dto.shutter_speed),
                        date: Set(shot_dto.date),
                        date_fuzzy: Set(shot_dto.date_fuzzy),
                        location: Set(shot_dto.location),
                        notes: Set(shot_dto.notes),
                        created_at: Set(now.clone()),
                        updated_at: Set(now.clone()),
                        ..Default::default()
                    };
                    let shot_result = shot_model.insert(txn).await?;

                    // Link lenses if provided
                    if let Some(lens_ids) = shot_dto.lens_ids {
                        if !lens_ids.is_empty() {
                            ShotService::set_lenses_for_shot(txn, shot_result.id, lens_ids)
                                .await?;
                        }
                    }
                }

                Ok(new_roll_id)
            })
        })
        .await
        .map_err(|e| {
            log::error!("Failed to import roll: {e}");
            format!("Could not import roll: {e}")
        })?;

    Ok(roll_id)
}
