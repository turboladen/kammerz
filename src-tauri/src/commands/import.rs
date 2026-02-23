use sea_orm::Set;

use crate::patch::{trim, trim_opt};
use serde::Deserialize;
use tauri::State;

use crate::entities::roll::{self, PushPull, RollStatus};
use crate::services::import_service::{ImportService, ModelInfo, ParsedRoll};
use crate::services::roll_service::{ImportShotEntry, RollService};
use crate::services::settings_service::SettingsService;
use crate::AppState;

const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct ImportRollDto {
    pub roll_id: String,
    pub camera_id: Option<i32>,
    pub film_stock_id: Option<i32>,
    pub lens_id: Option<i32>,
    pub status: RollStatus,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub date_fuzzy: Option<String>,
    pub push_pull: Option<PushPull>,
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

    let roll_model = roll::ActiveModel {
        roll_id: trim(data.roll_id),
        camera_id: Set(data.camera_id),
        film_stock_id: Set(data.film_stock_id),
        lens_id: Set(data.lens_id),
        status: Set(data.status),
        frame_count: Set(data.frame_count),
        date_loaded: trim_opt(data.date_loaded),
        date_finished: trim_opt(data.date_finished),
        date_fuzzy: trim_opt(data.date_fuzzy),
        push_pull: Set(data.push_pull),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };

    let shot_entries: Vec<ImportShotEntry> = data
        .shots
        .into_iter()
        .map(|s| ImportShotEntry {
            frame_number: s.frame_number.trim().to_string(),
            aperture: s.aperture.map(|v| v.trim().to_string()),
            shutter_speed: s.shutter_speed.map(|v| v.trim().to_string()),
            date: s.date.map(|v| v.trim().to_string()),
            date_fuzzy: s.date_fuzzy.map(|v| v.trim().to_string()),
            location: s.location.map(|v| v.trim().to_string()),
            notes: s.notes.map(|v| v.trim().to_string()),
            lens_ids: s.lens_ids,
        })
        .collect();

    RollService::import_roll(&state.db, roll_model, shot_entries)
        .await
        .map_err(|e| {
            log::error!("Failed to import roll: {e}");
            format!("Could not import roll: {e}")
        })
}
