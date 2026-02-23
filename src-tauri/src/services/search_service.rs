use sea_orm::*;
use serde::Serialize;

pub struct SearchService;

// --- Result types ---

#[derive(Debug, Serialize, FromQueryResult)]
pub struct CameraSearchResult {
    pub id: i32,
    pub brand: String,
    pub model: String,
    pub format: String,
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
    pub format: String,
    pub stock_type: String,
    pub match_field: String,
    pub match_snippet: String,
}

#[derive(Debug, Serialize, FromQueryResult)]
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
    pub async fn search(
        db: &DatabaseConnection,
        query: &str,
    ) -> Result<SearchResults, DbErr> {
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

        let rolls = RollSearchResult::find_by_statement(Statement::from_sql_and_values(
            backend,
            r#"SELECT r.id, r.roll_id, r.status,
                c.brand AS camera_brand, c.model AS camera_model,
                fs.brand AS film_stock_brand, fs.name AS film_stock_name,
                CASE
                    WHEN r.roll_id LIKE $1 THEN 'roll ID'
                    WHEN r.status LIKE $1 THEN 'status'
                    WHEN r.date_fuzzy LIKE $1 THEN 'date note'
                    WHEN r.push_pull LIKE $1 THEN 'push/pull'
                    WHEN r.notes LIKE $1 THEN 'notes'
                    ELSE 'unknown'
                END AS match_field,
                CASE
                    WHEN r.roll_id LIKE $1 THEN r.roll_id
                    WHEN r.status LIKE $1 THEN r.status
                    WHEN r.date_fuzzy LIKE $1 THEN COALESCE(r.date_fuzzy, '')
                    WHEN r.push_pull LIKE $1 THEN COALESCE(r.push_pull, '')
                    WHEN r.notes LIKE $1 THEN COALESCE(r.notes, '')
                    ELSE ''
                END AS match_snippet
            FROM rolls r
            LEFT JOIN cameras c ON r.camera_id = c.id
            LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id
            WHERE r.roll_id LIKE $1 OR r.status LIKE $1 OR r.date_fuzzy LIKE $1
                OR r.push_pull LIKE $1 OR r.notes LIKE $1
            LIMIT 20"#,
            [pattern.clone().into()],
        ))
        .all(db)
        .await?;

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
