use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult, OptionExt};
use crate::extract::{Json, Path};
use crate::patch::{double_option, now_string, trim_opt};
use crate::routes::{friendly_delete_err, friendly_err};
use crate::services::development_service::DevelopmentService;
use crate::services::roll_service::{RollService, RollWithDetails};
use crate::services::shot_service::ShotService;
use crate::validate::{require_nonempty, validate_date_opt, validate_non_negative_i32};
use crate::AppState;
use entity::roll::{self, PushPull, RollStatus};
use entity::{dev_stage, development_lab, development_self, shot};

// --- DTOs (moved verbatim from commands/rolls.rs) ---

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
    pub date_scanned: Option<String>,
    pub date_post_processed: Option<String>,
    pub date_archived: Option<String>,
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
    pub date_scanned: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_post_processed: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_archived: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub date_fuzzy: Option<Option<String>>,
    #[serde(deserialize_with = "double_option")]
    pub push_pull: Option<Option<PushPull>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

// --- Composite roll detail (reduces round-trips) ---

#[derive(Debug, Serialize)]
pub struct RollDetail {
    pub roll: RollWithDetails,
    pub shots: Vec<shot::Model>,
    pub shot_lens_pairs: Vec<(i32, i32)>,
    pub lab_dev: Option<development_lab::Model>,
    pub self_dev: Option<development_self::Model>,
    pub dev_stages: Vec<dev_stage::Model>,
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
) -> AppResult<Json<Vec<RollWithDetails>>> {
    Ok(Json(RollService::list_all_with_details(&db).await?))
}

async fn get_one(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Option<RollWithDetails>>> {
    Ok(Json(RollService::get_with_details(&db, id).await?))
}

async fn create(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateRollDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date_loaded", &data.date_loaded)?;
    validate_date_opt("date_finished", &data.date_finished)?;
    validate_date_opt("date_scanned", &data.date_scanned)?;
    validate_date_opt("date_post_processed", &data.date_post_processed)?;
    validate_date_opt("date_archived", &data.date_archived)?;
    let roll_id = require_nonempty("roll_id", &data.roll_id)?;
    validate_non_negative_i32("frame_count", data.frame_count)?;

    let now = now_string();
    let model = roll::ActiveModel {
        roll_id: Set(roll_id),
        camera_id: Set(data.camera_id),
        film_stock_id: Set(data.film_stock_id),
        lens_id: Set(data.lens_id),
        status: Set(data.status),
        frame_count: Set(data.frame_count),
        date_loaded: trim_opt(data.date_loaded),
        date_finished: trim_opt(data.date_finished),
        date_scanned: trim_opt(data.date_scanned),
        date_post_processed: trim_opt(data.date_post_processed),
        date_archived: trim_opt(data.date_archived),
        date_fuzzy: trim_opt(data.date_fuzzy),
        push_pull: Set(data.push_pull),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let result = RollService::create(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("roll", e)))?;
    Ok((StatusCode::CREATED, Json(result.id)))
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
    if let Some(v) = &data.date_scanned {
        validate_date_opt("date_scanned", v)?;
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
    if let Some(v) = data.status {
        model.status = Set(v);
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
    if let Some(v) = data.date_scanned {
        model.date_scanned = trim_opt(v);
    }
    if let Some(v) = data.date_post_processed {
        model.date_post_processed = trim_opt(v);
    }
    if let Some(v) = data.date_archived {
        model.date_archived = trim_opt(v);
    }
    if let Some(v) = data.date_fuzzy {
        model.date_fuzzy = trim_opt(v);
    }
    if let Some(v) = data.push_pull {
        model.push_pull = Set(v);
    }
    if let Some(v) = data.notes {
        model.notes = trim_opt(v);
    }
    model.updated_at = Set(now);

    RollService::update(&db, model)
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
) -> AppResult<Json<Vec<RollWithDetails>>> {
    Ok(Json(RollService::list_for_camera(&db, camera_id).await?))
}

async fn suggest_id(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<String>> {
    Ok(Json(RollService::suggest_id(&db).await?))
}

// --- Composite roll detail (ported from commands/rolls.rs::get_roll_detail) ---

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

    Ok(Json(RollDetail {
        roll,
        shots,
        shot_lens_pairs,
        lab_dev,
        self_dev,
        dev_stages,
    }))
}
