use sea_orm::Set;
use serde::Deserialize;
use tauri::State;

use crate::entities::film_stock::{self, FilmFormat, FilmStockType};
use crate::patch::{double_option, trim, trim_opt};
use crate::services::film_stock_service::FilmStockService;
use crate::AppState;

// --- DTOs ---

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

// --- Commands ---

#[tauri::command]
pub async fn list_film_stocks(
    state: State<'_, AppState>,
) -> Result<Vec<film_stock::Model>, String> {
    FilmStockService::list_all(&state.db).await.map_err(|e| {
        log::error!("Failed to list film stocks: {e}");
        format!("Could not list film stocks: {e}")
    })
}

#[tauri::command]
pub async fn get_film_stock(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<film_stock::Model>, String> {
    FilmStockService::get_by_id(&state.db, id)
        .await
        .map_err(|e| {
            log::error!("Failed to get film stock {id}: {e}");
            format!("Could not get film stock: {e}")
        })
}

#[tauri::command]
pub async fn create_film_stock(
    state: State<'_, AppState>,
    data: CreateFilmStockDto,
) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = film_stock::ActiveModel {
        brand: trim(data.brand),
        name: trim(data.name),
        format: Set(data.format),
        exposure_count: Set(data.exposure_count),
        stock_type: Set(data.stock_type),
        iso: Set(data.iso),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let result = FilmStockService::create(&state.db, model)
        .await
        .map_err(|e| {
            log::error!("Failed to create film stock: {e}");
            super::friendly_err("film stock", e)
        })?;
    Ok(result.id)
}

#[tauri::command]
pub async fn update_film_stock(
    state: State<'_, AppState>,
    id: i32,
    data: UpdateFilmStockDto,
) -> Result<(), String> {
    let existing = FilmStockService::get_by_id(&state.db, id)
        .await
        .map_err(|e| format!("Could not find film stock: {e}"))?
        .ok_or_else(|| format!("Film stock {id} not found"))?;

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: film_stock::ActiveModel = existing.into();

    if let Some(v) = data.brand { model.brand = trim(v); }
    if let Some(v) = data.name { model.name = trim(v); }
    if let Some(v) = data.format { model.format = Set(v); }
    if let Some(v) = data.exposure_count { model.exposure_count = Set(v); }
    if let Some(v) = data.stock_type { model.stock_type = Set(v); }
    if let Some(v) = data.iso { model.iso = Set(v); }
    if let Some(v) = data.notes { model.notes = trim_opt(v); }
    model.updated_at = Set(now);

    FilmStockService::update(&state.db, model)
        .await
        .map_err(|e| {
            log::error!("Failed to update film stock {id}: {e}");
            super::friendly_err("film stock", e)
        })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_film_stock(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    FilmStockService::delete(&state.db, id).await.map_err(|e| {
        log::error!("Failed to delete film stock {id}: {e}");
        super::friendly_err("film stock", e)
    })
}

// --- Distinct value helpers ---

#[tauri::command]
pub async fn list_distinct_film_brands(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    FilmStockService::distinct_brands(&state.db)
        .await
        .map_err(|e| format!("Could not list film brands: {e}"))
}
