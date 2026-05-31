use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::patch::{double_option, trim, trim_opt};
use crate::routes::friendly_err;
use crate::services::development_service::DevelopmentService;
use crate::services::roll_service::{RollService, RollWithDetails};
use crate::services::shot_service::ShotService;
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
    RollService::list_all_with_details(&db)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn get_one(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Option<RollWithDetails>>> {
    RollService::get_with_details(&db, id)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn create(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Json(data): Json<CreateRollDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
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
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Roll {id} not found")))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: roll::ActiveModel = existing.into();

    if let Some(v) = data.roll_id {
        model.roll_id = trim(v);
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
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("roll", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_for_camera(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(camera_id): Path<i32>,
) -> AppResult<Json<Vec<RollWithDetails>>> {
    RollService::list_for_camera(&db, camera_id)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn suggest_id(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<Json<String>> {
    RollService::suggest_id(&db)
        .await
        .map(Json)
        .map_err(|e| AppError::Internal(e.to_string()))
}

// --- Composite roll detail (ported from commands/rolls.rs::get_roll_detail) ---

async fn get_detail(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<RollDetail>> {
    let roll = RollService::get_with_details(&db, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Roll {id} not found")))?;

    let shots = ShotService::list_for_roll(&db, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let shot_lens_pairs = ShotService::get_lenses_for_roll_shots(&db, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let lab_dev = DevelopmentService::get_lab_dev_for_roll(&db, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let self_dev = DevelopmentService::get_self_dev_for_roll(&db, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let dev_stages = if let Some(ref sd) = self_dev {
        DevelopmentService::list_stages(&db, sd.id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
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
