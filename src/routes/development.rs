use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, Set, TransactionTrait,
};
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::patch::{double_option, trim_opt};
use crate::routes::friendly_err;
use crate::services::development_service::{
    DevelopmentService, SelfDevWithStages, StageInput,
};
use crate::services::roll_service::RollService;
use crate::AppState;
use entity::roll::RollStatus;
use entity::{dev_stage, development_lab, development_self};

// --- DTOs (moved verbatim from commands/development.rs) ---

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

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/lab", post(create_lab_dev))
        .route("/lab/for-roll/{roll_id}", get(get_lab_dev_for_roll))
        .route(
            "/lab/{id}",
            axum::routing::put(update_lab_dev).delete(delete_lab_dev),
        )
        .route("/self", get(list_all_self_developments).post(create_self_dev))
        .route("/self/for-roll/{roll_id}", get(get_self_dev_for_roll))
        .route("/self/{id}/stages", get(list_dev_stages))
        .route(
            "/self/{id}",
            axum::routing::put(update_self_dev).delete(delete_self_dev),
        )
}

// --- Lab Development handlers ---

async fn get_lab_dev_for_roll(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(roll_id): Path<i32>,
) -> AppResult<Json<Option<development_lab::Model>>> {
    DevelopmentService::get_lab_dev_for_roll(&db, roll_id)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn create_lab_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateLabDevDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let result_id = db
        .transaction::<_, i32, DbErr>(|txn| {
            Box::pin(async move {
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
                let result = model.insert(txn).await?;

                // Auto-advance: → at-lab when lab dev record is created
                RollService::auto_sync_status(
                    txn,
                    data.roll_id,
                    &[RollStatus::Loaded, RollStatus::Shooting, RollStatus::Shot],
                    RollStatus::AtLab,
                )
                .await?;

                Ok(result.id)
            })
        })
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lab development", e)))?;

    Ok((StatusCode::CREATED, Json(result_id)))
}

async fn update_lab_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateLabDevDto>,
) -> AppResult<StatusCode> {
    let existing = DevelopmentService::get_lab_dev_by_id(&db, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Lab development {id} not found")))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: development_lab::ActiveModel = existing.into();

    if let Some(v) = data.lab_id {
        model.lab_id = Set(v);
    }
    if let Some(v) = data.date_dropped_off {
        model.date_dropped_off = trim_opt(v);
    }
    if let Some(v) = data.date_received {
        model.date_received = trim_opt(v);
    }
    if let Some(v) = data.cost {
        model.cost = Set(v);
    }
    if let Some(v) = data.notes {
        model.notes = trim_opt(v);
    }
    model.updated_at = Set(now);

    DevelopmentService::update_lab_dev(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("lab development", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_lab_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            // Look up dev record to get roll_id before deleting
            let dev = development_lab::Entity::find_by_id(id)
                .one(txn)
                .await?
                .ok_or_else(|| DbErr::Custom(format!("Lab development {id} not found")))?;
            let roll_id = dev.roll_id;

            // Delete the dev record
            development_lab::Entity::delete_by_id(id).exec(txn).await?;

            // Auto-revert: at-lab/lab-done → shot when lab dev is removed
            // (only if no self-dev record exists — sibling dev takes priority)
            let has_self_dev = development_self::Entity::find()
                .filter(development_self::Column::RollId.eq(roll_id))
                .count(txn)
                .await?
                > 0;

            if !has_self_dev {
                RollService::auto_sync_status(
                    txn,
                    roll_id,
                    &[RollStatus::AtLab, RollStatus::LabDone],
                    RollStatus::Shot,
                )
                .await?;
            }

            Ok(())
        })
    })
    .await
    .map_err(|e| AppError::UnprocessableEntity(friendly_err("lab development", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Self Development handlers ---

async fn get_self_dev_for_roll(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(roll_id): Path<i32>,
) -> AppResult<Json<Option<development_self::Model>>> {
    DevelopmentService::get_self_dev_for_roll(&db, roll_id)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn create_self_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateSelfDevDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let result_id = db
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

                // Auto-advance: → developing when self dev record is created
                RollService::auto_sync_status(
                    txn,
                    data.roll_id,
                    &[RollStatus::Loaded, RollStatus::Shooting, RollStatus::Shot],
                    RollStatus::Developing,
                )
                .await?;

                Ok(result.id)
            })
        })
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("self development", e)))?;

    Ok((StatusCode::CREATED, Json(result_id)))
}

async fn update_self_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateSelfDevDto>,
) -> AppResult<StatusCode> {
    let existing = DevelopmentService::get_self_dev_by_id(&db, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Self development {id} not found")))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            let mut model: development_self::ActiveModel = existing.into();

            if let Some(v) = data.date_processed {
                model.date_processed = trim_opt(v);
            }
            if let Some(v) = data.developer {
                model.developer = trim_opt(v);
            }
            if let Some(v) = data.developer_dilution {
                model.developer_dilution = trim_opt(v);
            }
            if let Some(v) = data.fixer {
                model.fixer = trim_opt(v);
            }
            if let Some(v) = data.fixer_dilution {
                model.fixer_dilution = trim_opt(v);
            }
            if let Some(v) = data.stop_bath {
                model.stop_bath = trim_opt(v);
            }
            if let Some(v) = data.wetting_agent {
                model.wetting_agent = trim_opt(v);
            }
            if let Some(v) = data.clearing_agent {
                model.clearing_agent = trim_opt(v);
            }
            if let Some(v) = data.temperature {
                model.temperature = trim_opt(v);
            }
            if let Some(v) = data.agitation_notes {
                model.agitation_notes = trim_opt(v);
            }
            if let Some(v) = data.notes {
                model.notes = trim_opt(v);
            }
            model.updated_at = Set(now);

            DevelopmentService::update_self_dev(txn, model).await?;

            if let Some(stages) = data.stages {
                DevelopmentService::set_stages(txn, id, stages_to_inputs(stages)).await?;
            }

            Ok(())
        })
    })
    .await
    .map_err(|e| AppError::UnprocessableEntity(friendly_err("self development", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_self_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            // Look up dev record to get roll_id before deleting
            let dev = development_self::Entity::find_by_id(id)
                .one(txn)
                .await?
                .ok_or_else(|| DbErr::Custom(format!("Self development {id} not found")))?;
            let roll_id = dev.roll_id;

            // Delete the dev record (dev stages cascade-deleted by FK)
            development_self::Entity::delete_by_id(id).exec(txn).await?;

            // Auto-revert: developing/developed → shot when self dev is removed
            // (only if no lab-dev record exists — sibling dev takes priority)
            let has_lab_dev = development_lab::Entity::find()
                .filter(development_lab::Column::RollId.eq(roll_id))
                .count(txn)
                .await?
                > 0;

            if !has_lab_dev {
                RollService::auto_sync_status(
                    txn,
                    roll_id,
                    &[RollStatus::Developing, RollStatus::Developed],
                    RollStatus::Shot,
                )
                .await?;
            }

            Ok(())
        })
    })
    .await
    .map_err(|e| AppError::UnprocessableEntity(friendly_err("self development", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Dev Stages handler ---

async fn list_dev_stages(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(development_self_id): Path<i32>,
) -> AppResult<Json<Vec<dev_stage::Model>>> {
    DevelopmentService::list_stages(&db, development_self_id)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

// --- List all self-developments (composite: parents + batched stages) ---

async fn list_all_self_developments(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<Vec<SelfDevWithStages>>> {
    let items = DevelopmentService::list_all_self_devs(&db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let dev_ids: Vec<i32> = items.iter().map(|i| i.dev_id).collect();

    let all_stages = DevelopmentService::list_stages_for_dev_ids(&db, dev_ids)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut stage_map: HashMap<i32, Vec<dev_stage::Model>> = HashMap::new();
    for stage in all_stages {
        stage_map
            .entry(stage.development_self_id)
            .or_default()
            .push(stage);
    }

    let result: Vec<SelfDevWithStages> = items
        .into_iter()
        .map(|item| {
            let stages = stage_map.remove(&item.dev_id).unwrap_or_default();
            SelfDevWithStages { item, stages }
        })
        .collect();

    Ok(Json(result))
}
