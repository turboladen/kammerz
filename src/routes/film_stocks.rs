use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use sea_orm::Set;
use serde::Deserialize;

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult, OptionExt};
use crate::patch::{double_option, now_string, trim_opt};
use crate::routes::{friendly_delete_err, friendly_err};
use crate::services::film_stock_service::FilmStockService;
use crate::validate::{require_nonempty, validate_non_negative_i32};
use crate::AppState;
use entity::film_stock::{self, FilmFormat, FilmStockType};

// --- DTOs (moved verbatim from commands/film_stocks.rs) ---

#[derive(Debug, Deserialize)]
pub struct CreateFilmStockDto {
    pub brand: String,
    pub name: String,
    pub format: FilmFormat,
    pub exposure_count: Option<i32>,
    pub stock_type: FilmStockType,
    pub iso: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateFilmStockDto {
    pub brand: Option<String>,
    pub name: Option<String>,
    pub format: Option<FilmFormat>,
    #[serde(deserialize_with = "double_option")]
    pub exposure_count: Option<Option<i32>>,
    pub stock_type: Option<FilmStockType>,
    #[serde(deserialize_with = "double_option")]
    pub iso: Option<Option<i32>>,
    #[serde(deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/distinct/brands", get(distinct_brands))
        .route("/{id}", get(get_one).put(update).delete(delete_one))
}

// --- Handlers ---

async fn list(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<Vec<film_stock::Model>>> {
    Ok(Json(FilmStockService::list_all(&db).await?))
}

async fn get_one(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<Json<Option<film_stock::Model>>> {
    Ok(Json(FilmStockService::get_by_id(&db, id).await?))
}

async fn create(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Json(data): Json<CreateFilmStockDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    let brand = require_nonempty("brand", &data.brand)?;
    let name = require_nonempty("name", &data.name)?;
    validate_non_negative_i32("exposure_count", data.exposure_count)?;
    validate_non_negative_i32("iso", data.iso)?;

    let now = now_string();
    let model = film_stock::ActiveModel {
        brand: Set(brand),
        name: Set(name),
        format: Set(data.format),
        exposure_count: Set(data.exposure_count),
        stock_type: Set(data.stock_type),
        iso: Set(data.iso),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let res = FilmStockService::create(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("film stock", e)))?;
    Ok((StatusCode::CREATED, Json(res.id)))
}

async fn update(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
    Json(data): Json<UpdateFilmStockDto>,
) -> AppResult<StatusCode> {
    let existing = FilmStockService::get_by_id(&db, id)
        .await?
        .or_404("Film stock", id)?;
    if let Some(v) = data.exposure_count {
        validate_non_negative_i32("exposure_count", v)?;
    }
    if let Some(v) = data.iso {
        validate_non_negative_i32("iso", v)?;
    }
    let now = now_string();
    let mut model: film_stock::ActiveModel = existing.into();
    if let Some(v) = data.brand {
        model.brand = Set(require_nonempty("brand", &v)?);
    }
    if let Some(v) = data.name {
        model.name = Set(require_nonempty("name", &v)?);
    }
    if let Some(v) = data.format {
        model.format = Set(v);
    }
    if let Some(v) = data.exposure_count {
        model.exposure_count = Set(v);
    }
    if let Some(v) = data.stock_type {
        model.stock_type = Set(v);
    }
    if let Some(v) = data.iso {
        model.iso = Set(v);
    }
    if let Some(v) = data.notes {
        model.notes = trim_opt(v);
    }
    model.updated_at = Set(now);
    FilmStockService::update(&db, model)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("film stock", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_one(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
    Path(id): Path<i32>,
) -> AppResult<StatusCode> {
    FilmStockService::delete(&db, id)
        .await
        .map_err(|e| friendly_delete_err("film stock", e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn distinct_brands(
    _: RequireAuth,
    State(db): State<sea_orm::DatabaseConnection>,
) -> AppResult<Json<Vec<String>>> {
    Ok(Json(FilmStockService::distinct_brands(&db).await?))
}
