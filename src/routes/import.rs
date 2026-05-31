use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use sea_orm::{DatabaseConnection, Set};
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::patch::{trim, trim_opt};
use crate::routes::friendly_err;
use crate::services::import_service::{ImportService, ModelInfo, ParsedRoll};
use crate::services::roll_service::{ImportShotEntry, RollService};
use crate::services::settings_service::SettingsService;
use crate::AppState;
use entity::roll::{self, PushPull, RollStatus};

const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";

// --- DTOs (moved verbatim from commands/import.rs) ---

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

#[derive(Debug, Deserialize)]
pub struct ParseNoteDto {
    pub note_text: String,
    pub model: Option<String>,
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/models", get(list_models))
        .route("/parse", post(parse_note))
        .route("/roll", post(import_parsed_roll))
}

// --- Key resolution ---

/// Resolve the Anthropic API key: prefer the server config (env
/// `ANTHROPIC_API_KEY`), then fall back to the `claude_api_key` settings row.
async fn resolve_key(db: &DatabaseConnection, config: &AppConfig) -> AppResult<String> {
    if let Some(k) = &config.anthropic_api_key {
        return Ok(k.clone());
    }
    SettingsService::get_setting(db, "claude_api_key")
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            AppError::UnprocessableEntity(
                "No Anthropic API key configured. Set it in Settings or the ANTHROPIC_API_KEY env var."
                    .into(),
            )
        })
}

// --- Handlers ---

async fn list_models(
    _: RequireAuth,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ModelInfo>>> {
    let key = resolve_key(&state.db, &state.config).await?;
    ImportService::list_models(&key)
        .await
        .map(Json)
        .map_err(AppError::UnprocessableEntity)
}

async fn parse_note(
    _: RequireAuth,
    State(state): State<AppState>,
    Json(data): Json<ParseNoteDto>,
) -> AppResult<Json<ParsedRoll>> {
    let key = resolve_key(&state.db, &state.config).await?;
    let model = match data.model {
        Some(m) if !m.is_empty() => m,
        _ => SettingsService::get_setting(&state.db, "claude_model")
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
    };
    ImportService::parse_note(&key, &model, &data.note_text)
        .await
        .map(Json)
        .map_err(AppError::UnprocessableEntity)
}

async fn import_parsed_roll(
    _: RequireAuth,
    State(state): State<AppState>,
    Json(data): Json<ImportRollDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

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

    let id = RollService::import_roll(&state.db, roll_model, shot_entries)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("roll", e)))?;
    Ok((StatusCode::CREATED, Json(id)))
}
