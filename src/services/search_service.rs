use sea_orm::*;
use serde::Serialize;

use ::entity::camera::CameraFormat;
use ::entity::film_stock::{FilmFormat, FilmStockType};

use crate::activity::{ActivitySignals, legacy_status};

pub struct SearchService;

// --- Result types ---

#[derive(Debug, Serialize, FromQueryResult)]
pub struct CameraSearchResult {
    pub id: i32,
    pub brand: String,
    pub model: String,
    pub format: CameraFormat,
    pub match_field: String,
    pub match_snippet: String,
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct LensSearchResult {
    pub id: i32,
    pub brand: String,
    pub model: Option<String>,
    pub focal_length: Option<String>,
    pub match_field: String,
    pub match_snippet: String,
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct FilmStockSearchResult {
    pub id: i32,
    pub brand: String,
    pub name: String,
    pub format: FilmFormat,
    pub stock_type: FilmStockType,
    pub match_field: String,
    pub match_snippet: String,
}

/// A roll search hit. `status` is the compat legacy status, derived in Rust from
/// the roll's dates (ADR-0013) — there is no stored status column to select.
#[derive(Debug, Serialize)]
pub struct RollSearchResult {
    pub id: i32,
    pub roll_id: String,
    pub status: String,
    pub camera_brand: Option<String>,
    pub camera_model: Option<String>,
    pub film_stock_brand: Option<String>,
    pub film_stock_name: Option<String>,
    pub match_field: String,
    pub match_snippet: String,
}

/// Raw row for the roll search query, including the activity-derivation signals.
#[derive(Debug, FromQueryResult)]
struct RollSearchRow {
    id: i32,
    roll_id: String,
    camera_brand: Option<String>,
    camera_model: Option<String>,
    film_stock_brand: Option<String>,
    film_stock_name: Option<String>,
    date_loaded: Option<String>,
    date_finished: Option<String>,
    date_scanned: Option<String>,
    date_post_processed: Option<String>,
    date_archived: Option<String>,
    archive_na: bool,
    shot_count: i64,
    lab_dev_id: Option<i32>,
    lab_completion: Option<String>,
    self_dev_id: Option<i32>,
    self_completion: Option<String>,
    match_field: String,
    match_snippet: String,
}

impl From<RollSearchRow> for RollSearchResult {
    fn from(row: RollSearchRow) -> Self {
        let is_lab_dev = row.lab_dev_id.is_some();
        let sig = ActivitySignals {
            shot_count: row.shot_count,
            date_loaded: row.date_loaded,
            date_finished: row.date_finished,
            has_dev: is_lab_dev || row.self_dev_id.is_some(),
            is_lab_dev,
            dev_started: None,
            dev_completion: if is_lab_dev {
                row.lab_completion
            } else {
                row.self_completion
            },
            scan_started: None,
            date_scanned: row.date_scanned,
            post_processing_started: None,
            date_post_processed: row.date_post_processed,
            date_archived: row.date_archived,
            archive_na: row.archive_na,
        };
        RollSearchResult {
            id: row.id,
            roll_id: row.roll_id,
            status: legacy_status(&sig),
            camera_brand: row.camera_brand,
            camera_model: row.camera_model,
            film_stock_brand: row.film_stock_brand,
            film_stock_name: row.film_stock_name,
            match_field: row.match_field,
            match_snippet: row.match_snippet,
        }
    }
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct ShotSearchResult {
    pub id: i32,
    pub frame_number: String,
    pub roll_pk: i32,
    pub roll_id_display: String,
    pub aperture: Option<String>,
    pub location: Option<String>,
    pub match_field: String,
    pub match_snippet: String,
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct LabSearchResult {
    pub id: i32,
    pub name: String,
    pub location: Option<String>,
    pub match_field: String,
    pub match_snippet: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub cameras: Vec<CameraSearchResult>,
    pub lenses: Vec<LensSearchResult>,
    pub film_stocks: Vec<FilmStockSearchResult>,
    pub rolls: Vec<RollSearchResult>,
    pub shots: Vec<ShotSearchResult>,
    pub labs: Vec<LabSearchResult>,
}

impl SearchService {
    pub async fn search(db: &DatabaseConnection, query: &str) -> Result<SearchResults, DbErr> {
        let pattern = format!("%{query}%");
        let backend = db.get_database_backend();

        let cameras = CameraSearchResult::find_by_statement(Statement::from_sql_and_values(
            backend,
            r#"SELECT id, brand, model, format,
                CASE
                    WHEN brand LIKE $1 THEN 'brand'
                    WHEN model LIKE $1 THEN 'model'
                    WHEN prefix LIKE $1 THEN 'prefix'
                    WHEN format LIKE $1 THEN 'format'
                    WHEN camera_type LIKE $1 THEN 'type'
                    WHEN serial_number LIKE $1 THEN 'serial number'
                    WHEN notes LIKE $1 THEN 'notes'
                    ELSE 'unknown'
                END AS match_field,
                CASE
                    WHEN brand LIKE $1 THEN brand
                    WHEN model LIKE $1 THEN model
                    WHEN prefix LIKE $1 THEN COALESCE(prefix, '')
                    WHEN format LIKE $1 THEN format
                    WHEN camera_type LIKE $1 THEN COALESCE(camera_type, '')
                    WHEN serial_number LIKE $1 THEN COALESCE(serial_number, '')
                    WHEN notes LIKE $1 THEN COALESCE(notes, '')
                    ELSE ''
                END AS match_snippet
            FROM cameras
            WHERE brand LIKE $1 OR model LIKE $1 OR prefix LIKE $1 OR format LIKE $1
                OR camera_type LIKE $1 OR serial_number LIKE $1 OR notes LIKE $1
            LIMIT 20"#,
            [pattern.clone().into()],
        ))
        .all(db)
        .await?;

        let lenses = LensSearchResult::find_by_statement(Statement::from_sql_and_values(
            backend,
            r#"SELECT id, brand, model, focal_length,
                CASE
                    WHEN brand LIKE $1 THEN 'brand'
                    WHEN model LIKE $1 THEN 'model'
                    WHEN focal_length LIKE $1 THEN 'focal length'
                    WHEN lens_system LIKE $1 THEN 'system'
                    WHEN max_aperture LIKE $1 THEN 'aperture'
                    WHEN serial_number LIKE $1 THEN 'serial number'
                    WHEN notes LIKE $1 THEN 'notes'
                    ELSE 'unknown'
                END AS match_field,
                CASE
                    WHEN brand LIKE $1 THEN brand
                    WHEN model LIKE $1 THEN COALESCE(model, '')
                    WHEN focal_length LIKE $1 THEN COALESCE(focal_length, '')
                    WHEN lens_system LIKE $1 THEN COALESCE(lens_system, '')
                    WHEN max_aperture LIKE $1 THEN COALESCE(max_aperture, '')
                    WHEN serial_number LIKE $1 THEN COALESCE(serial_number, '')
                    WHEN notes LIKE $1 THEN COALESCE(notes, '')
                    ELSE ''
                END AS match_snippet
            FROM lenses
            WHERE brand LIKE $1 OR model LIKE $1 OR focal_length LIKE $1
                OR lens_system LIKE $1 OR max_aperture LIKE $1 OR serial_number LIKE $1
                OR notes LIKE $1
            LIMIT 20"#,
            [pattern.clone().into()],
        ))
        .all(db)
        .await?;

        let film_stocks = FilmStockSearchResult::find_by_statement(Statement::from_sql_and_values(
            backend,
            r#"SELECT id, brand, name, format, stock_type,
                CASE
                    WHEN brand LIKE $1 THEN 'brand'
                    WHEN name LIKE $1 THEN 'name'
                    WHEN format LIKE $1 THEN 'format'
                    WHEN stock_type LIKE $1 THEN 'type'
                    WHEN notes LIKE $1 THEN 'notes'
                    ELSE 'unknown'
                END AS match_field,
                CASE
                    WHEN brand LIKE $1 THEN brand
                    WHEN name LIKE $1 THEN name
                    WHEN format LIKE $1 THEN format
                    WHEN stock_type LIKE $1 THEN stock_type
                    WHEN notes LIKE $1 THEN COALESCE(notes, '')
                    ELSE ''
                END AS match_snippet
            FROM film_stocks
            WHERE brand LIKE $1 OR name LIKE $1 OR format LIKE $1
                OR stock_type LIKE $1 OR notes LIKE $1
            LIMIT 20"#,
            [pattern.clone().into()],
        ))
        .all(db)
        .await?;

        // `status` is no longer a column (ADR-0013), so it is neither matched nor
        // selected: search matches roll_id / push_pull / notes, and the compat
        // status is derived per row from the activity signals below.
        let rolls: Vec<RollSearchResult> =
            RollSearchRow::find_by_statement(Statement::from_sql_and_values(
                backend,
                r#"SELECT r.id, r.roll_id,
                c.brand AS camera_brand, c.model AS camera_model,
                fs.brand AS film_stock_brand, fs.name AS film_stock_name,
                r.date_loaded, r.date_finished, r.date_scanned,
                r.date_post_processed, r.date_archived, r.archive_na,
                (SELECT COUNT(*) FROM shots s WHERE s.roll_id = r.id) AS shot_count,
                dl.id AS lab_dev_id, dl.date_received AS lab_completion,
                ds.id AS self_dev_id, ds.date_processed AS self_completion,
                CASE
                    WHEN r.roll_id LIKE $1 THEN 'roll ID'
                    WHEN r.push_pull LIKE $1 THEN 'push/pull'
                    WHEN r.notes LIKE $1 THEN 'notes'
                    ELSE 'unknown'
                END AS match_field,
                CASE
                    WHEN r.roll_id LIKE $1 THEN r.roll_id
                    WHEN r.push_pull LIKE $1 THEN COALESCE(r.push_pull, '')
                    WHEN r.notes LIKE $1 THEN COALESCE(r.notes, '')
                    ELSE ''
                END AS match_snippet
            FROM rolls r
            LEFT JOIN cameras c ON r.camera_id = c.id
            LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id
            LEFT JOIN development_labs dl ON dl.roll_id = r.id
            LEFT JOIN development_selves ds ON ds.roll_id = r.id
            WHERE r.roll_id LIKE $1
                OR r.push_pull LIKE $1 OR r.notes LIKE $1
            LIMIT 20"#,
                [pattern.clone().into()],
            ))
            .all(db)
            .await?
            .into_iter()
            .map(RollSearchResult::from)
            .collect();

        let shots = ShotSearchResult::find_by_statement(Statement::from_sql_and_values(
            backend,
            r#"SELECT s.id, s.frame_number, s.roll_id AS roll_pk,
                r.roll_id AS roll_id_display,
                s.aperture, s.location,
                CASE
                    WHEN s.frame_number LIKE $1 THEN 'frame'
                    WHEN s.aperture LIKE $1 THEN 'aperture'
                    WHEN s.shutter_speed LIKE $1 THEN 'shutter speed'
                    WHEN s.location LIKE $1 THEN 'location'
                    WHEN s.notes LIKE $1 THEN 'notes'
                    ELSE 'unknown'
                END AS match_field,
                CASE
                    WHEN s.frame_number LIKE $1 THEN s.frame_number
                    WHEN s.aperture LIKE $1 THEN COALESCE(s.aperture, '')
                    WHEN s.shutter_speed LIKE $1 THEN COALESCE(s.shutter_speed, '')
                    WHEN s.location LIKE $1 THEN COALESCE(s.location, '')
                    WHEN s.notes LIKE $1 THEN COALESCE(s.notes, '')
                    ELSE ''
                END AS match_snippet
            FROM shots s
            JOIN rolls r ON s.roll_id = r.id
            WHERE s.frame_number LIKE $1 OR s.aperture LIKE $1 OR s.shutter_speed LIKE $1
                OR s.location LIKE $1 OR s.notes LIKE $1
            LIMIT 20"#,
            [pattern.clone().into()],
        ))
        .all(db)
        .await?;

        let labs = LabSearchResult::find_by_statement(Statement::from_sql_and_values(
            backend,
            r#"SELECT id, name, location,
                CASE
                    WHEN name LIKE $1 THEN 'name'
                    WHEN location LIKE $1 THEN 'location'
                    WHEN website LIKE $1 THEN 'website'
                    WHEN notes LIKE $1 THEN 'notes'
                    ELSE 'unknown'
                END AS match_field,
                CASE
                    WHEN name LIKE $1 THEN name
                    WHEN location LIKE $1 THEN COALESCE(location, '')
                    WHEN website LIKE $1 THEN COALESCE(website, '')
                    WHEN notes LIKE $1 THEN COALESCE(notes, '')
                    ELSE ''
                END AS match_snippet
            FROM labs
            WHERE name LIKE $1 OR location LIKE $1 OR website LIKE $1 OR notes LIKE $1
            LIMIT 20"#,
            [pattern.into()],
        ))
        .all(db)
        .await?;

        Ok(SearchResults {
            cameras,
            lenses,
            film_stocks,
            rolls,
            shots,
            labs,
        })
    }
}
