use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, Set, TransactionTrait};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::activity::RollActivity;
use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult, OptionExt};
use crate::extract::{Json, Path};
use crate::patch::{double_option, now_string, trim_opt};
use crate::routes::{friendly_delete_err, friendly_err};
use crate::services::development_service::DevelopmentService;
use crate::services::roll_event_service::RollEventService;
use crate::services::roll_service::{RollService, RollWithDetails};
use crate::services::shot_service::ShotService;
use crate::validate::{require_nonempty, validate_date_opt, validate_non_negative_i32};
use entity::roll::{self, PushPull};
use entity::{dev_stage, development_lab, development_self, shot};

// --- DTOs ---

#[derive(Debug, Deserialize)]
pub struct CreateRollDto {
    pub roll_id: String,
    pub camera_id: Option<i32>,
    pub film_stock_id: Option<i32>,
    pub lens_id: Option<i32>,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub scan_started: Option<String>,
    pub date_scanned: Option<String>,
    pub post_processing_started: Option<String>,
    pub date_post_processed: Option<String>,
    pub date_archived: Option<String>,
    pub archive_location: Option<String>,
    pub archive_na: Option<bool>,
    pub archive_na_reason: Option<String>,
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
    #[serde(deserialize_with = "double_option")]
    pub frame_count: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub date_loaded: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_finished: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub scan_started: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_scanned: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub post_processing_started: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_post_processed: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_archived: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub archive_location: Option<Option<String>>,
    pub archive_na: Option<bool>,
    #[serde(deserialize_with = "double_option")]
    pub archive_na_reason: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub push_pull: Option<Option<PushPull>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

// --- Response view: a roll row plus its server-derived activity fields ---

/// A roll list/detail row with the derived activity view flattened in
/// (`activities`, `badge`, `group_key`, `done`, compat `status`) so the frontend
/// never re-derives the lifecycle (ADR-0013).
#[derive(Debug, Serialize)]
pub struct RollView {
    #[serde(flatten)]
    pub roll: RollWithDetails,
    #[serde(flatten)]
    pub activity: RollActivity,
}

impl From<RollWithDetails> for RollView {
    fn from(roll: RollWithDetails) -> Self {
        let activity = roll.activity();
        RollView { roll, activity }
    }
}

// --- Composite roll detail (reduces round-trips) ---

#[derive(Debug, Serialize)]
pub struct RollDetail {
    pub roll: RollView,
    pub shots: Vec<shot::Model>,
    pub shot_lens_pairs: Vec<(i32, i32)>,
    pub lab_dev: Option<development_lab::Model>,
    pub self_dev: Option<development_self::Model>,
    pub dev_stages: Vec<dev_stage::Model>,
    pub events: Vec<entity::roll_event::Model>,
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/suggest-id", get(suggest_id))
        .route("/for-camera/{camera_id}", get(list_for_camera))
        .route("/{id}", get(get_one).put(update).delete(delete_one))
        .route("/{id}/detail", get(get_detail))
}

// --- Handlers ---

async fn list(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<Vec<RollView>>> {
    let rolls = RollService::list_all_with_details(&db).await?;
    Ok(Json(rolls.into_iter().map(RollView::from).collect()))
}

async fn get_one(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Option<RollView>>> {
    let roll = RollService::get_with_details(&db, id).await?;
    Ok(Json(roll.map(RollView::from)))
}

async fn create(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateRollDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date_loaded", &data.date_loaded)?;
    validate_date_opt("date_finished", &data.date_finished)?;
    validate_date_opt("scan_started", &data.scan_started)?;
    validate_date_opt("date_scanned", &data.date_scanned)?;
    validate_date_opt("post_processing_started", &data.post_processing_started)?;
    validate_date_opt("date_post_processed", &data.date_post_processed)?;
    validate_date_opt("date_archived", &data.date_archived)?;
    let roll_id = require_nonempty("roll_id", &data.roll_id)?;
    validate_non_negative_i32("frame_count", data.frame_count)?;

    // Archiving is done XOR N/A (ADR-0013): a recorded archive date wins and
    // clears the N/A flag.
    let has_archive_date = data
        .date_archived
        .as_ref()
        .is_some_and(|s| !s.trim().is_empty());
    let archive_na = data.archive_na.unwrap_or(false) && !has_archive_date;

    let now = now_string();
    let model = roll::ActiveModel {
        roll_id: Set(roll_id),
        camera_id: Set(data.camera_id),
        film_stock_id: Set(data.film_stock_id),
        lens_id: Set(data.lens_id),
        frame_count: Set(data.frame_count),
        date_loaded: trim_opt(data.date_loaded),
        date_finished: trim_opt(data.date_finished),
        scan_started: trim_opt(data.scan_started),
        date_scanned: trim_opt(data.date_scanned),
        post_processing_started: trim_opt(data.post_processing_started),
        date_post_processed: trim_opt(data.date_post_processed),
        date_archived: trim_opt(data.date_archived),
        archive_location: trim_opt(data.archive_location),
        archive_na: Set(archive_na),
        archive_na_reason: trim_opt(data.archive_na_reason),
        push_pull: Set(data.push_pull),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let result_id = db
        .transaction::<_, i32, DbErr>(|txn| {
            Box::pin(async move {
                let result = RollService::create(txn, model).await?;
                RollEventService::record(
                    txn,
                    result.id,
                    entity::roll_event::RollEventType::RollLoaded,
                    None,
                    None,
                    "Roll loaded".to_string(),
                )
                .await?;
                Ok(result.id)
            })
        })
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("roll", e)))?;
    Ok((StatusCode::CREATED, Json(result_id)))
}

async fn update(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateRollDto>,
) -> AppResult<StatusCode> {
    let existing = roll::Entity::find_by_id(id)
        .one(&db)
        .await?
        .or_404("Roll", id)?;

    if let Some(v) = &data.date_loaded {
        validate_date_opt("date_loaded", v)?;
    }
    if let Some(v) = &data.date_finished {
        validate_date_opt("date_finished", v)?;
    }
    if let Some(v) = &data.scan_started {
        validate_date_opt("scan_started", v)?;
    }
    if let Some(v) = &data.date_scanned {
        validate_date_opt("date_scanned", v)?;
    }
    if let Some(v) = &data.post_processing_started {
        validate_date_opt("post_processing_started", v)?;
    }
    if let Some(v) = &data.date_post_processed {
        validate_date_opt("date_post_processed", v)?;
    }
    if let Some(v) = &data.date_archived {
        validate_date_opt("date_archived", v)?;
    }
    if let Some(v) = data.frame_count {
        validate_non_negative_i32("frame_count", v)?;
    }

    let now = now_string();
    let mut model: roll::ActiveModel = existing.into();

    if let Some(v) = data.roll_id {
        model.roll_id = Set(require_nonempty("roll_id", &v)?);
    }
    if let Some(v) = data.camera_id {
        model.camera_id = Set(v);
    }
    if let Some(v) = data.film_stock_id {
        model.film_stock_id = Set(v);
    }
    if let Some(v) = data.lens_id {
        model.lens_id = Set(v);
    }
    if let Some(v) = data.frame_count {
        model.frame_count = Set(v);
    }
    if let Some(v) = data.date_loaded {
        model.date_loaded = trim_opt(v);
    }
    if let Some(v) = data.date_finished {
        model.date_finished = trim_opt(v);
    }
    if let Some(v) = data.scan_started {
        model.scan_started = trim_opt(v);
    }
    if let Some(v) = data.date_scanned {
        model.date_scanned = trim_opt(v);
    }
    if let Some(v) = data.post_processing_started {
        model.post_processing_started = trim_opt(v);
    }
    if let Some(v) = data.date_post_processed {
        model.date_post_processed = trim_opt(v);
    }
    if let Some(v) = data.date_archived {
        model.date_archived = trim_opt(v);
    }
    if let Some(v) = data.archive_location {
        model.archive_location = trim_opt(v);
    }
    if let Some(v) = data.archive_na {
        model.archive_na = Set(v);
    }
    if let Some(v) = data.archive_na_reason {
        model.archive_na_reason = trim_opt(v);
    }
    if let Some(v) = data.push_pull {
        model.push_pull = Set(v);
    }
    if let Some(v) = data.notes {
        model.notes = trim_opt(v);
    }

    // Archiving is done XOR N/A (ADR-0013): whichever this update sets wins and
    // clears the other. A recorded archive date takes priority.
    let sets_archive_date = matches!(&model.date_archived, Set(Some(s)) if !s.trim().is_empty());
    if sets_archive_date {
        model.archive_na = Set(false);
    } else if matches!(&model.archive_na, Set(true)) {
        model.date_archived = Set(None);
    }

    model.updated_at = Set(now);

    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            RollService::update(txn, model).await?;
            Ok(())
        })
    })
    .await
    .map_err(|e| AppError::UnprocessableEntity(friendly_err("roll", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_one(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    RollService::delete(&db, id)
        .await
        .map_err(|e| friendly_delete_err("roll", e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_for_camera(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(camera_id): Path<i32>,
) -> AppResult<Json<Vec<RollView>>> {
    let rolls = RollService::list_for_camera(&db, camera_id).await?;
    Ok(Json(rolls.into_iter().map(RollView::from).collect()))
}

async fn suggest_id(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<String>> {
    Ok(Json(RollService::suggest_id(&db).await?))
}

// --- Composite roll detail ---

async fn get_detail(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<RollDetail>> {
    let roll = RollService::get_with_details(&db, id)
        .await?
        .or_404("Roll", id)?;

    let shots = ShotService::list_for_roll(&db, id).await?;

    let shot_lens_pairs = ShotService::get_lenses_for_roll_shots(&db, id).await?;

    let lab_dev = DevelopmentService::get_lab_dev_for_roll(&db, id).await?;

    let self_dev = DevelopmentService::get_self_dev_for_roll(&db, id).await?;

    let dev_stages = if let Some(ref sd) = self_dev {
        DevelopmentService::list_stages(&db, sd.id).await?
    } else {
        vec![]
    };

    let events = RollEventService::list_for_roll(&db, id).await?;

    Ok(Json(RollDetail {
        roll: RollView::from(roll),
        shots,
        shot_lens_pairs,
        lab_dev,
        self_dev,
        dev_stages,
        events,
    }))
}
