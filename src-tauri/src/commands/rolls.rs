use sea_orm::{EntityTrait, Set};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::entities::roll::{self, PushPull, RollStatus};
use crate::entities::{dev_stage, development_lab, development_self, shot};
use crate::patch::{double_option, trim, trim_opt};
use crate::services::development_service::DevelopmentService;
use crate::services::roll_service::{RollService, RollWithDetails};
use crate::services::shot_service::ShotService;
use crate::AppState;

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct CreateRollDto {
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
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateRollDto {
    pub roll_id: Option<String>,
    #[serde(deserialize_with = "double_option")]
    pub camera_id: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub film_stock_id: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub lens_id: Option<Option<i32>>,
    pub status: Option<RollStatus>,
    #[serde(deserialize_with = "double_option")]
    pub frame_count: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub date_loaded: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_finished: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_fuzzy: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub push_pull: Option<Option<PushPull>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
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
    let result = RollService::create(&state.db, model).await.map_err(|e| {
        log::error!("Failed to create roll: {e}");
        super::friendly_err("roll", e)
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

    if let Some(v) = data.roll_id { model.roll_id = trim(v); }
    if let Some(v) = data.camera_id { model.camera_id = Set(v); }
    if let Some(v) = data.film_stock_id { model.film_stock_id = Set(v); }
    if let Some(v) = data.lens_id { model.lens_id = Set(v); }
    if let Some(v) = data.status { model.status = Set(v); }
    if let Some(v) = data.frame_count { model.frame_count = Set(v); }
    if let Some(v) = data.date_loaded { model.date_loaded = trim_opt(v); }
    if let Some(v) = data.date_finished { model.date_finished = trim_opt(v); }
    if let Some(v) = data.date_fuzzy { model.date_fuzzy = trim_opt(v); }
    if let Some(v) = data.push_pull { model.push_pull = Set(v); }
    if let Some(v) = data.notes { model.notes = trim_opt(v); }
    model.updated_at = Set(now);

    RollService::update(&state.db, model).await.map_err(|e| {
        log::error!("Failed to update roll {id}: {e}");
        super::friendly_err("roll", e)
    })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_roll(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    RollService::delete(&state.db, id).await.map_err(|e| {
        log::error!("Failed to delete roll {id}: {e}");
        super::friendly_err("roll", e)
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

// --- Composite roll detail (reduces IPC round-trips) ---

#[derive(Debug, Serialize)]
pub struct RollDetail {
    pub roll: RollWithDetails,
    pub shots: Vec<shot::Model>,
    pub shot_lens_pairs: Vec<(i32, i32)>,
    pub lab_dev: Option<development_lab::Model>,
    pub self_dev: Option<development_self::Model>,
    pub dev_stages: Vec<dev_stage::Model>,
}

#[tauri::command]
pub async fn get_roll_detail(
    state: State<'_, AppState>,
    id: i32,
) -> Result<RollDetail, String> {
    let roll = RollService::get_with_details(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to get roll detail {id}: {e}");
            format!("Could not load roll: {e}")
        })?
        .ok_or_else(|| format!("Roll {id} not found"))?;

    let shots = ShotService::list_for_roll(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to load shots for roll {id}: {e}");
            format!("Could not load shots: {e}")
        })?;

    let shot_lens_pairs = ShotService::get_lenses_for_roll_shots(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to load shot lenses for roll {id}: {e}");
            format!("Could not load shot lenses: {e}")
        })?;

    let lab_dev = DevelopmentService::get_lab_dev_for_roll(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to load lab dev for roll {id}: {e}");
            format!("Could not load lab development: {e}")
        })?;

    let self_dev = DevelopmentService::get_self_dev_for_roll(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to load self dev for roll {id}: {e}");
            format!("Could not load self development: {e}")
        })?;

    let dev_stages = if let Some(ref sd) = self_dev {
        DevelopmentService::list_stages(&state.db, sd.id)
            .await
            .map_err(|e| {
                log::error!("Failed to load dev stages for roll {id}: {e}");
                format!("Could not load development stages: {e}")
            })?
    } else {
        vec![]
    };

    Ok(RollDetail {
        roll,
        shots,
        shot_lens_pairs,
        lab_dev,
        self_dev,
        dev_stages,
    })
}
