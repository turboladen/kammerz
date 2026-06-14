use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;

use crate::AppState;
use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult, DbOptionExt, OptionExt};
use crate::extract::{Json, Path};
use crate::patch::{double_option, now_string, trim, trim_opt};
use crate::routes::{Op, friendly_err, friendly_txn_err};
use crate::services::roll_event_service::RollEventService;
use crate::services::roll_service::RollService;
use crate::services::shot_service::ShotService;
use crate::validate::{require_nonempty, validate_date_opt, validate_lat, validate_lon};
use entity::roll::RollStatus;
use entity::shot;

// --- DTOs (moved verbatim from commands/shots.rs) ---

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

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create))
        .route("/for-roll/{roll_id}", get(list_for_roll))
        .route("/for-roll/{roll_id}/lenses", get(lenses_for_roll_shots))
        .route("/for-roll/{roll_id}/count", get(count_for_roll))
        .route("/for-roll/{roll_id}/next-frame", get(suggest_next_frame))
        .route("/{id}", get(get_one).put(update).delete(delete_one))
        .route("/{id}/lenses", get(lenses_for_shot))
}

// --- Handlers ---

async fn list_for_roll(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(roll_id): Path<i32>,
) -> AppResult<Json<Vec<shot::Model>>> {
    Ok(Json(ShotService::list_for_roll(&db, roll_id).await?))
}

async fn get_one(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Option<shot::Model>>> {
    Ok(Json(ShotService::get_by_id(&db, id).await?))
}

// --- Create shot (transactional: create + set_lenses + auto_sync_status) ---

async fn create(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateShotDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date", &data.date)?;
    require_nonempty("frame_number", &data.frame_number)?;
    validate_lat("gps_lat", data.gps_lat)?;
    validate_lon("gps_lon", data.gps_lon)?;

    let now = now_string();

    let result_id = db
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

                // Auto-advance: loaded → shooting when first shot is added
                RollService::auto_sync_status(
                    txn,
                    data.roll_id,
                    &[RollStatus::Loaded],
                    RollStatus::Shooting,
                )
                .await?;

                RollEventService::record(
                    txn,
                    data.roll_id,
                    entity::roll_event::RollEventType::ShotLogged,
                    None,
                    None,
                    Some(entity::roll_event::RefKind::Shot),
                    Some(result.id),
                    format!("Frame {} logged", result.frame_number),
                )
                .await?;

                Ok(result.id)
            })
        })
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("shot", e)))?;

    Ok((StatusCode::CREATED, Json(result_id)))
}

// --- Update shot (transactional: update + set_lenses) ---

async fn update(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateShotDto>,
) -> AppResult<StatusCode> {
    let existing = ShotService::get_by_id(&db, id).await?.or_404("Shot", id)?;
    let roll_id = existing.roll_id;

    if let Some(v) = &data.date {
        validate_date_opt("date", v)?;
    }
    if let Some(v) = &data.frame_number {
        require_nonempty("frame_number", v)?;
    }
    if let Some(v) = data.gps_lat {
        validate_lat("gps_lat", v)?;
    }
    if let Some(v) = data.gps_lon {
        validate_lon("gps_lon", v)?;
    }

    let now = now_string();

    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            let mut model: shot::ActiveModel = existing.into();

            if let Some(v) = data.frame_number {
                model.frame_number = trim(v);
            }
            if let Some(v) = data.aperture {
                model.aperture = trim_opt(v);
            }
            if let Some(v) = data.shutter_speed {
                model.shutter_speed = trim_opt(v);
            }
            if let Some(v) = data.date {
                model.date = trim_opt(v);
            }
            if let Some(v) = data.date_fuzzy {
                model.date_fuzzy = trim_opt(v);
            }
            if let Some(v) = data.location {
                model.location = trim_opt(v);
            }
            if let Some(v) = data.gps_lat {
                model.gps_lat = Set(v);
            }
            if let Some(v) = data.gps_lon {
                model.gps_lon = Set(v);
            }
            if let Some(v) = data.notes {
                model.notes = trim_opt(v);
            }
            model.updated_at = Set(now);

            ShotService::update(txn, model).await?;

            if let Some(lens_ids) = data.lens_ids {
                ShotService::set_lenses_for_shot(txn, id, lens_ids).await?;
            }

            RollEventService::record(
                txn,
                roll_id,
                entity::roll_event::RollEventType::ShotEdited,
                None,
                None,
                Some(entity::roll_event::RefKind::Shot),
                Some(id),
                "Shot edited".to_string(),
            )
            .await?;

            Ok(())
        })
    })
    .await
    .map_err(|e| AppError::UnprocessableEntity(friendly_err("shot", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Delete shot (transactional: delete + auto_sync_status revert) ---

async fn delete_one(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            // Look up shot to get roll_id before deleting
            let shot_record = shot::Entity::find_by_id(id)
                .one(txn)
                .await?
                .or_404_db("Shot", id)?;
            let roll_id = shot_record.roll_id;

            // Delete the shot (shot_lenses cascade-deleted by FK)
            shot::Entity::delete_by_id(id).exec(txn).await?;

            // Auto-revert: shooting/shot → loaded when last shot is removed
            let remaining = shot::Entity::find()
                .filter(shot::Column::RollId.eq(roll_id))
                .count(txn)
                .await?;

            if remaining == 0 {
                RollService::auto_sync_status(
                    txn,
                    roll_id,
                    &[RollStatus::Shooting, RollStatus::Shot],
                    RollStatus::Loaded,
                )
                .await?;
            }

            RollEventService::record(
                txn,
                roll_id,
                entity::roll_event::RollEventType::ShotDeleted,
                None,
                None,
                None,
                None,
                format!("Frame {} deleted", shot_record.frame_number),
            )
            .await?;

            Ok(())
        })
    })
    .await
    .map_err(|e| friendly_txn_err("shot", Op::Delete, e))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn lenses_for_shot(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(shot_id): Path<i32>,
) -> AppResult<Json<Vec<i32>>> {
    Ok(Json(ShotService::get_lenses_for_shot(&db, shot_id).await?))
}

async fn lenses_for_roll_shots(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(roll_id): Path<i32>,
) -> AppResult<Json<Vec<(i32, i32)>>> {
    Ok(Json(
        ShotService::get_lenses_for_roll_shots(&db, roll_id).await?,
    ))
}

async fn suggest_next_frame(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(roll_id): Path<i32>,
) -> AppResult<Json<String>> {
    Ok(Json(ShotService::suggest_next_frame(&db, roll_id).await?))
}

async fn count_for_roll(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(roll_id): Path<i32>,
) -> AppResult<Json<u64>> {
    Ok(Json(ShotService::count_for_roll(&db, roll_id).await?))
}
