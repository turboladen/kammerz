use std::collections::HashMap;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, Set, TransactionTrait,
};
use serde::Deserialize;

use crate::AppState;
use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult, DbOptionExt, OptionExt};
use crate::extract::{Json, Path};
use crate::patch::{double_option, now_string, trim_opt};
use crate::routes::{Op, friendly_err, friendly_txn_err};
use crate::services::chemical_service::{ChemicalService, GroupedChemicals};
use crate::services::development_service::{
    DevelopmentService, LabDevListItem, SelfDevWithStages, StageInput,
};
use crate::services::roll_event_service::RollEventService;
use crate::validate::{
    require_nonempty, validate_date_opt, validate_non_negative_f64, validate_non_negative_i32,
};
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
    #[serde(deserialize_with = "double_option")]
    pub date_negatives_picked_up: Option<Option<String>>,
    pub negatives_not_collecting: Option<bool>,
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

/// Validate each stage before the rows are written: `stage_name` must not be
/// empty (it is `NOT NULL`), and `duration_seconds` / `sort_order` must be
/// non-negative. Names the offending index so the 422 points the user at the
/// right stage row.
fn validate_stages(stages: &[StageDto]) -> AppResult<()> {
    for (i, s) in stages.iter().enumerate() {
        require_nonempty(&format!("stages[{i}].stage_name"), &s.stage_name)?;
        validate_non_negative_i32(&format!("stages[{i}].duration_seconds"), s.duration_seconds)?;
        validate_non_negative_i32(&format!("stages[{i}].sort_order"), Some(s.sort_order))?;
    }
    Ok(())
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
        .route("/lab", get(list_all_lab_developments).post(create_lab_dev))
        .route("/lab/for-roll/{roll_id}", get(get_lab_dev_for_roll))
        .route(
            "/lab/{id}",
            axum::routing::put(update_lab_dev).delete(delete_lab_dev),
        )
        .route(
            "/self",
            get(list_all_self_developments).post(create_self_dev),
        )
        .route("/self/for-roll/{roll_id}", get(get_self_dev_for_roll))
        .route("/self/{id}/stages", get(list_dev_stages))
        .route(
            "/self/{id}",
            axum::routing::put(update_self_dev).delete(delete_self_dev),
        )
        .route("/chemicals", get(list_chemicals))
}

// --- Chemistry reference handler ---

/// Canonical chemistry reference grouped by type, for the self-dev autocomplete.
async fn list_chemicals(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<GroupedChemicals>> {
    Ok(Json(ChemicalService::list_grouped(&db).await?))
}

// --- Lab Development handlers ---

// List every lab dev with joined roll/film stock/camera/lab context. Lab devs
// carry no stages, so (unlike list_all_self_developments) the rows are returned
// flat with no batch-merge step.
async fn list_all_lab_developments(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<Vec<LabDevListItem>>> {
    Ok(Json(DevelopmentService::list_all_lab_devs(&db).await?))
}

async fn get_lab_dev_for_roll(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(roll_id): Path<i32>,
) -> AppResult<Json<Option<development_lab::Model>>> {
    Ok(Json(
        DevelopmentService::get_lab_dev_for_roll(&db, roll_id).await?,
    ))
}

async fn create_lab_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateLabDevDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date_dropped_off", &data.date_dropped_off)?;
    validate_date_opt("date_received", &data.date_received)?;
    validate_non_negative_f64("cost", data.cost)?;

    let now = now_string();

    let result_id = db
        .transaction::<_, i32, DbErr>(|txn| {
            Box::pin(async move {
                // Lab and self dev are mutually exclusive (core lifecycle
                // invariant). The UI hides the "+ Lab" button once a self dev
                // exists, but a stale tab on another device (or a raw API call)
                // can still POST — enforce inside the transaction so the check
                // and insert are atomic. DbErr::Custom carries the friendly
                // message through friendly_txn_err verbatim as a 422.
                let has_self_dev = development_self::Entity::find()
                    .filter(development_self::Column::RollId.eq(data.roll_id))
                    .count(txn)
                    .await?
                    > 0;
                if has_self_dev {
                    return Err(DbErr::Custom(
                        "This roll already has a self development record — delete it first."
                            .to_string(),
                    ));
                }

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

                // The Development activity now derives from this record's dates
                // (ADR-0013) — no stored status to advance or adopt.
                RollEventService::record(
                    txn,
                    data.roll_id,
                    entity::roll_event::RollEventType::LabDevAdded,
                    Some(entity::roll_event::RefKind::LabDev),
                    Some(result.id),
                    "Lab development added".to_string(),
                )
                .await?;

                Ok(result.id)
            })
        })
        .await
        .map_err(|e| friendly_txn_err("lab development", Op::Write, e))?;

    Ok((StatusCode::CREATED, Json(result_id)))
}

async fn update_lab_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateLabDevDto>,
) -> AppResult<StatusCode> {
    let existing = DevelopmentService::get_lab_dev_by_id(&db, id)
        .await?
        .or_404("Lab development", id)?;

    if let Some(v) = &data.date_dropped_off {
        validate_date_opt("date_dropped_off", v)?;
    }
    if let Some(v) = &data.date_received {
        validate_date_opt("date_received", v)?;
    }
    if let Some(v) = data.cost {
        validate_non_negative_f64("cost", v)?;
    }
    if let Some(v) = &data.date_negatives_picked_up {
        validate_date_opt("date_negatives_picked_up", v)?;
    }

    let now = now_string();

    // Which specialized negatives action (if any) this edit performs — captured
    // before `existing` is consumed. Pickup takes priority over waive over a
    // plain edit for the journal entry.
    let picking_up = matches!(&data.date_negatives_picked_up, Some(Some(s)) if !s.trim().is_empty())
        && existing.date_negatives_picked_up.is_none();
    let waiving = data.negatives_not_collecting == Some(true) && !existing.negatives_not_collecting;

    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
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
            if let Some(v) = data.date_negatives_picked_up {
                model.date_negatives_picked_up = trim_opt(v);
            }
            if let Some(v) = data.negatives_not_collecting {
                model.negatives_not_collecting = Set(v);
            }
            model.updated_at = Set(now);

            let result = DevelopmentService::update_lab_dev(txn, model).await?;

            // The Development/Scanning activities derive from this record's dates
            // (ADR-0013) — no stored status to reconcile.
            let (event_type, summary) = if picking_up {
                (
                    entity::roll_event::RollEventType::NegativesPickedUp,
                    "Negatives picked up".to_string(),
                )
            } else if waiving {
                (
                    entity::roll_event::RollEventType::NegativesWaived,
                    "Negatives marked not for collection".to_string(),
                )
            } else {
                (
                    entity::roll_event::RollEventType::LabDevEdited,
                    "Lab development edited".to_string(),
                )
            };
            RollEventService::record(
                txn,
                result.roll_id,
                event_type,
                Some(entity::roll_event::RefKind::LabDev),
                Some(id),
                summary,
            )
            .await?;

            Ok(())
        })
    })
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
                .or_404_db("Lab development", id)?;
            let roll_id = dev.roll_id;

            // Delete the dev record. The Development activity re-derives from the
            // (now absent) record (ADR-0013) — no stored status to revert.
            development_lab::Entity::delete_by_id(id).exec(txn).await?;

            RollEventService::record(
                txn,
                roll_id,
                entity::roll_event::RollEventType::LabDevRemoved,
                None,
                None,
                "Lab development removed".to_string(),
            )
            .await?;

            Ok(())
        })
    })
    .await
    .map_err(|e| friendly_txn_err("lab development", Op::Delete, e))?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Self Development handlers ---

async fn get_self_dev_for_roll(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(roll_id): Path<i32>,
) -> AppResult<Json<Option<development_self::Model>>> {
    Ok(Json(
        DevelopmentService::get_self_dev_for_roll(&db, roll_id).await?,
    ))
}

async fn create_self_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateSelfDevDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date_processed", &data.date_processed)?;
    if let Some(stages) = &data.stages {
        validate_stages(stages)?;
    }

    let now = now_string();

    let result_id = db
        .transaction::<_, i32, DbErr>(|txn| {
            Box::pin(async move {
                // Mirror of the create_lab_dev guard: lab and self dev are
                // mutually exclusive, and the UI-only enforcement can be bypassed
                // by a stale tab on another device or a raw API call.
                let has_lab_dev = development_lab::Entity::find()
                    .filter(development_lab::Column::RollId.eq(data.roll_id))
                    .count(txn)
                    .await?
                    > 0;
                if has_lab_dev {
                    return Err(DbErr::Custom(
                        "This roll already has a lab development record — delete it first."
                            .to_string(),
                    ));
                }

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

                // Self-learning: record any novel chemistry values so they become
                // future autocomplete suggestions (kammerz-9fx). Same transaction.
                ChemicalService::upsert_from_self_dev(txn, &result).await?;

                if let Some(stages) = data.stages {
                    DevelopmentService::set_stages(txn, result.id, stages_to_inputs(stages))
                        .await?;
                }

                // The Development activity now derives from this record's dates
                // (ADR-0013) — no stored status to advance or adopt.
                RollEventService::record(
                    txn,
                    data.roll_id,
                    entity::roll_event::RollEventType::SelfDevAdded,
                    Some(entity::roll_event::RefKind::SelfDev),
                    Some(result.id),
                    "Self development added".to_string(),
                )
                .await?;

                Ok(result.id)
            })
        })
        .await
        .map_err(|e| friendly_txn_err("self development", Op::Write, e))?;

    Ok((StatusCode::CREATED, Json(result_id)))
}

async fn update_self_dev(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateSelfDevDto>,
) -> AppResult<StatusCode> {
    let existing = DevelopmentService::get_self_dev_by_id(&db, id)
        .await?
        .or_404("Self development", id)?;

    if let Some(v) = &data.date_processed {
        validate_date_opt("date_processed", v)?;
    }
    if let Some(stages) = &data.stages {
        validate_stages(stages)?;
    }

    let now = now_string();

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

            let result = DevelopmentService::update_self_dev(txn, model).await?;

            // Self-learning: record any novel chemistry values so they become
            // future autocomplete suggestions (kammerz-9fx). Same transaction.
            ChemicalService::upsert_from_self_dev(txn, &result).await?;

            if let Some(stages) = data.stages {
                DevelopmentService::set_stages(txn, id, stages_to_inputs(stages)).await?;
            }

            // The Development/Scanning activities derive from this record's dates
            // (ADR-0013) — no stored status to reconcile.
            RollEventService::record(
                txn,
                result.roll_id,
                entity::roll_event::RollEventType::SelfDevEdited,
                Some(entity::roll_event::RefKind::SelfDev),
                Some(id),
                "Self development edited".to_string(),
            )
            .await?;

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
                .or_404_db("Self development", id)?;
            let roll_id = dev.roll_id;

            // Delete the dev record (dev stages cascade-deleted by FK). The
            // Development activity re-derives from the (now absent) record
            // (ADR-0013) — no stored status to revert.
            development_self::Entity::delete_by_id(id).exec(txn).await?;

            RollEventService::record(
                txn,
                roll_id,
                entity::roll_event::RollEventType::SelfDevRemoved,
                None,
                None,
                "Self development removed".to_string(),
            )
            .await?;

            Ok(())
        })
    })
    .await
    .map_err(|e| friendly_txn_err("self development", Op::Delete, e))?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Dev Stages handler ---

async fn list_dev_stages(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(development_self_id): Path<i32>,
) -> AppResult<Json<Vec<dev_stage::Model>>> {
    Ok(Json(
        DevelopmentService::list_stages(&db, development_self_id).await?,
    ))
}

// --- List all self-developments (composite: parents + batched stages) ---

async fn list_all_self_developments(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<Vec<SelfDevWithStages>>> {
    let items = DevelopmentService::list_all_self_devs(&db).await?;

    let dev_ids: Vec<i32> = items.iter().map(|i| i.dev_id).collect();

    let all_stages = DevelopmentService::list_stages_for_dev_ids(&db, dev_ids).await?;

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
