use std::collections::HashMap;

use sea_orm::{DbErr, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::entities::{dev_stage, development_lab, development_self};
use crate::patch::{double_option, trim_opt};
use crate::services::development_service::{DevelopmentService, SelfDevListItem, StageInput};
use crate::AppState;

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct CreateLabDevDto {
    pub roll_id: i32,
    pub lab_id: Option<i32>,
    pub date_dropped_off: Option<String>,
    pub date_received: Option<String>,
    pub cost: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateLabDevDto {
    #[serde(deserialize_with = "double_option")]
    pub lab_id: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub date_dropped_off: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_received: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub cost: Option<Option<f64>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSelfDevDto {
    pub roll_id: i32,
    pub date_processed: Option<String>,
    pub developer: Option<String>,
    pub developer_dilution: Option<String>,
    pub fixer: Option<String>,
    pub fixer_dilution: Option<String>,
    pub stop_bath: Option<String>,
    pub wetting_agent: Option<String>,
    pub clearing_agent: Option<String>,
    pub temperature: Option<String>,
    pub agitation_notes: Option<String>,
    pub notes: Option<String>,
    pub stages: Option<Vec<StageDto>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateSelfDevDto {
    #[serde(deserialize_with = "double_option")]
    pub date_processed: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub developer: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub developer_dilution: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub fixer: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub fixer_dilution: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub stop_bath: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub wetting_agent: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub clearing_agent: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub temperature: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub agitation_notes: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
    pub stages: Option<Vec<StageDto>>,
}

#[derive(Debug, Deserialize)]
pub struct StageDto {
    pub stage_name: String,
    pub duration_seconds: Option<i32>,
    pub notes: Option<String>,
    pub sort_order: i32,
}

fn stages_to_inputs(stages: Vec<StageDto>) -> Vec<StageInput> {
    stages
        .into_iter()
        .map(|s| StageInput {
            stage_name: s.stage_name.trim().to_string(),
            duration_seconds: s.duration_seconds,
            notes: s.notes.map(|n| n.trim().to_string()),
            sort_order: s.sort_order,
        })
        .collect()
}

// --- Lab Development Commands ---

#[tauri::command]
pub async fn get_lab_dev_for_roll(
    state: State<'_, AppState>,
    roll_id: i32,
) -> Result<Option<development_lab::Model>, String> {
    DevelopmentService::get_lab_dev_for_roll(&state.db, roll_id)
        .await
        .map_err(|e| {
            log::error!("Failed to get lab dev for roll {roll_id}: {e}");
            format!("Could not get lab development: {e}")
        })
}

#[tauri::command]
pub async fn create_lab_dev(
    state: State<'_, AppState>,
    data: CreateLabDevDto,
) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = development_lab::ActiveModel {
        roll_id: Set(data.roll_id),
        lab_id: Set(data.lab_id),
        date_dropped_off: trim_opt(data.date_dropped_off),
        date_received: trim_opt(data.date_received),
        cost: Set(data.cost),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let result = DevelopmentService::create_lab_dev(&state.db, model)
        .await
        .map_err(|e| {
            log::error!("Failed to create lab dev: {e}");
            super::friendly_err("lab development", e)
        })?;
    Ok(result.id)
}

#[tauri::command]
pub async fn update_lab_dev(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateLabDevDto,
) -> Result<(), String> {
    let existing = DevelopmentService::get_lab_dev_by_id(&state.db, id)
        .await
        .map_err(|e| format!("Could not find lab dev: {e}"))?
        .ok_or_else(|| format!("Lab development {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: development_lab::ActiveModel = existing.into();

    if let Some(v) = data.lab_id { model.lab_id = Set(v); }
    if let Some(v) = data.date_dropped_off { model.date_dropped_off = trim_opt(v); }
    if let Some(v) = data.date_received { model.date_received = trim_opt(v); }
    if let Some(v) = data.cost { model.cost = Set(v); }
    if let Some(v) = data.notes { model.notes = trim_opt(v); }
    model.updated_at = Set(now);

    DevelopmentService::update_lab_dev(&state.db, model)
        .await
        .map_err(|e| {
            log::error!("Failed to update lab dev {id}: {e}");
            super::friendly_err("lab development", e)
        })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_lab_dev(
    state: State<'_, AppState>,
    id: i32,
) -> Result<(), String> {
    DevelopmentService::delete_lab_dev(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to delete lab dev {id}: {e}");
            format!("Could not delete lab development: {e}")
        })
}

// --- Self Development Commands ---

#[tauri::command]
pub async fn get_self_dev_for_roll(
    state: State<'_, AppState>,
    roll_id: i32,
) -> Result<Option<development_self::Model>, String> {
    DevelopmentService::get_self_dev_for_roll(&state.db, roll_id)
        .await
        .map_err(|e| {
            log::error!("Failed to get self dev for roll {roll_id}: {e}");
            format!("Could not get self development: {e}")
        })
}

#[tauri::command]
pub async fn create_self_dev(
    state: State<'_, AppState>,
    data: CreateSelfDevDto,
) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let result_id = state
        .db
        .transaction::<_, i32, DbErr>(|txn| {
            Box::pin(async move {
                let model = development_self::ActiveModel {
                    roll_id: Set(data.roll_id),
                    date_processed: trim_opt(data.date_processed),
                    developer: trim_opt(data.developer),
                    developer_dilution: trim_opt(data.developer_dilution),
                    fixer: trim_opt(data.fixer),
                    fixer_dilution: trim_opt(data.fixer_dilution),
                    stop_bath: trim_opt(data.stop_bath),
                    wetting_agent: trim_opt(data.wetting_agent),
                    clearing_agent: trim_opt(data.clearing_agent),
                    temperature: trim_opt(data.temperature),
                    agitation_notes: trim_opt(data.agitation_notes),
                    notes: trim_opt(data.notes),
                    created_at: Set(now.clone()),
                    updated_at: Set(now),
                    ..Default::default()
                };
                let result = DevelopmentService::create_self_dev(txn, model).await?;

                if let Some(stages) = data.stages {
                    DevelopmentService::set_stages(txn, result.id, stages_to_inputs(stages))
                        .await?;
                }

                Ok(result.id)
            })
        })
        .await
        .map_err(|e| {
            log::error!("Failed to create self dev: {e}");
            super::friendly_err("self development", e)
        })?;

    Ok(result_id)
}

#[tauri::command]
pub async fn update_self_dev(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateSelfDevDto,
) -> Result<(), String> {
    let existing = DevelopmentService::get_self_dev_by_id(&state.db, id)
        .await
        .map_err(|e| format!("Could not find self dev: {e}"))?
        .ok_or_else(|| format!("Self development {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    state
        .db
        .transaction::<_, (), DbErr>(|txn| {
            Box::pin(async move {
                let mut model: development_self::ActiveModel = existing.into();

                if let Some(v) = data.date_processed { model.date_processed = trim_opt(v); }
                if let Some(v) = data.developer { model.developer = trim_opt(v); }
                if let Some(v) = data.developer_dilution { model.developer_dilution = trim_opt(v); }
                if let Some(v) = data.fixer { model.fixer = trim_opt(v); }
                if let Some(v) = data.fixer_dilution { model.fixer_dilution = trim_opt(v); }
                if let Some(v) = data.stop_bath { model.stop_bath = trim_opt(v); }
                if let Some(v) = data.wetting_agent { model.wetting_agent = trim_opt(v); }
                if let Some(v) = data.clearing_agent { model.clearing_agent = trim_opt(v); }
                if let Some(v) = data.temperature { model.temperature = trim_opt(v); }
                if let Some(v) = data.agitation_notes { model.agitation_notes = trim_opt(v); }
                if let Some(v) = data.notes { model.notes = trim_opt(v); }
                model.updated_at = Set(now);

                DevelopmentService::update_self_dev(txn, model).await?;

                if let Some(stages) = data.stages {
                    DevelopmentService::set_stages(txn, id, stages_to_inputs(stages)).await?;
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| {
            log::error!("Failed to update self dev {id}: {e}");
            super::friendly_err("self development", e)
        })?;

    Ok(())
}

#[tauri::command]
pub async fn delete_self_dev(
    state: State<'_, AppState>,
    id: i32,
) -> Result<(), String> {
    DevelopmentService::delete_self_dev(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to delete self dev {id}: {e}");
            format!("Could not delete self development: {e}")
        })
}

// --- Dev Stages Command ---

#[tauri::command]
pub async fn list_dev_stages(
    state: State<'_, AppState>,
    development_self_id: i32,
) -> Result<Vec<dev_stage::Model>, String> {
    DevelopmentService::list_stages(&state.db, development_self_id)
        .await
        .map_err(|e| {
            log::error!("Failed to list dev stages for {development_self_id}: {e}");
            format!("Could not list stages: {e}")
        })
}

// --- List all self-developments ---

#[derive(Debug, Serialize)]
pub struct SelfDevWithStages {
    #[serde(flatten)]
    pub item: SelfDevListItem,
    pub stages: Vec<dev_stage::Model>,
}

#[tauri::command]
pub async fn list_all_self_developments(
    state: State<'_, AppState>,
) -> Result<Vec<SelfDevWithStages>, String> {
    let items = DevelopmentService::list_all_self_devs(&state.db)
        .await
        .map_err(|e| {
            log::error!("Failed to list all self developments: {e}");
            format!("Could not list developments: {e}")
        })?;

    let dev_ids: Vec<i32> = items.iter().map(|i| i.dev_id).collect();

    let all_stages = DevelopmentService::list_stages_for_dev_ids(&state.db, dev_ids)
        .await
        .map_err(|e| {
            log::error!("Failed to load dev stages: {e}");
            format!("Could not load stages: {e}")
        })?;

    let mut stage_map: HashMap<i32, Vec<dev_stage::Model>> = HashMap::new();
    for stage in all_stages {
        stage_map
            .entry(stage.development_self_id)
            .or_default()
            .push(stage);
    }

    let result = items
        .into_iter()
        .map(|item| {
            let stages = stage_map.remove(&item.dev_id).unwrap_or_default();
            SelfDevWithStages { item, stages }
        })
        .collect();

    Ok(result)
}
