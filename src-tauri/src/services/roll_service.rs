use sea_orm::*;
use serde::Serialize;

use crate::entities::roll::{self, Entity as Roll};

/// Flat struct for rolls joined with camera and film stock data.
/// Mirrors the frontend's `RollWithDetails` TypeScript interface.
#[derive(Debug, Serialize, FromQueryResult)]
pub struct RollWithDetails {
    // Roll fields
    pub id: i32,
    pub roll_id: String,
    pub camera_id: Option<i32>,
    pub film_stock_id: Option<i32>,
    pub lens_id: Option<i32>,
    pub status: String,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub date_fuzzy: Option<String>,
    pub push_pull: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // Joined camera fields
    pub camera_brand: Option<String>,
    pub camera_model: Option<String>,
    // Joined film stock fields
    pub film_stock_brand: Option<String>,
    pub film_stock_name: Option<String>,
    pub film_stock_iso: Option<i32>,
    // Joined lens fields
    pub lens_brand: Option<String>,
    pub lens_name: Option<String>,
}

const ROLLS_WITH_DETAILS_SQL: &str = "\
    SELECT r.id, r.roll_id, r.camera_id, r.film_stock_id, r.lens_id, r.status, \
           r.frame_count, r.date_loaded, r.date_finished, r.date_fuzzy, \
           r.push_pull, r.notes, r.created_at, r.updated_at, \
           c.brand AS camera_brand, c.model AS camera_model, \
           fs.brand AS film_stock_brand, fs.name AS film_stock_name, \
           fs.iso AS film_stock_iso, \
           l.brand AS lens_brand, \
           COALESCE(l.name_on_lens, l.focal_length) AS lens_name \
    FROM rolls r \
    LEFT JOIN cameras c ON r.camera_id = c.id \
    LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id \
    LEFT JOIN lenses l ON r.lens_id = l.id";

pub struct RollService;

impl RollService {
    pub async fn list_all_with_details(
        db: &DatabaseConnection,
    ) -> Result<Vec<RollWithDetails>, DbErr> {
        let sql = format!("{ROLLS_WITH_DETAILS_SQL} ORDER BY r.created_at DESC");
        RollWithDetails::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            sql,
        ))
        .all(db)
        .await
    }

    pub async fn get_with_details(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<RollWithDetails>, DbErr> {
        let sql = format!("{ROLLS_WITH_DETAILS_SQL} WHERE r.id = $1");
        RollWithDetails::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [id.into()],
        ))
        .one(db)
        .await
    }

    pub async fn create(
        db: &DatabaseConnection,
        model: roll::ActiveModel,
    ) -> Result<roll::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &DatabaseConnection,
        model: roll::ActiveModel,
    ) -> Result<roll::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        Roll::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    pub async fn list_for_camera(
        db: &DatabaseConnection,
        camera_id: i32,
    ) -> Result<Vec<RollWithDetails>, DbErr> {
        let sql = format!(
            "{ROLLS_WITH_DETAILS_SQL} WHERE r.camera_id = $1 ORDER BY r.created_at DESC"
        );
        RollWithDetails::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [camera_id.into()],
        ))
        .all(db)
        .await
    }

    /// Suggest a roll ID in YYMMDD-N format.
    pub async fn suggest_id(db: &DatabaseConnection) -> Result<String, DbErr> {
        let now = chrono::Local::now();
        let prefix = now.format("%y%m%d").to_string();

        #[derive(Debug, FromQueryResult)]
        struct CountRow {
            count: i64,
        }

        let pattern = format!("{prefix}-%");
        let row = CountRow::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT COUNT(*) as count FROM rolls WHERE roll_id LIKE $1",
            [pattern.into()],
        ))
        .one(db)
        .await?;

        let next = row.map(|r| r.count).unwrap_or(0) + 1;
        Ok(format!("{prefix}-{next}"))
    }
}
