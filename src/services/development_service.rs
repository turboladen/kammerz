use sea_orm::*;
use serde::Serialize;

use ::entity::dev_stage::{self, Entity as DevStage};
use ::entity::development_lab::{self, Entity as DevelopmentLab};
use ::entity::development_self::{self, Entity as DevelopmentSelf};
use ::entity::film_stock::FilmStockType;
use ::entity::roll::RollStatus;

/// A self-development list item with its ordered stages merged in.
/// Returned by the `list_all_self_developments` endpoint.
#[derive(Debug, Serialize)]
pub struct SelfDevWithStages {
    #[serde(flatten)]
    pub item: SelfDevListItem,
    pub stages: Vec<dev_stage::Model>,
}

/// Flat struct for self-developments joined with roll, film stock, and camera data.
#[derive(Debug, Serialize, FromQueryResult)]
pub struct SelfDevListItem {
    pub dev_id: i32,
    pub roll_pk: i32,
    pub roll_id: String,
    pub roll_status: RollStatus,
    pub film_stock_brand: Option<String>,
    pub film_stock_name: Option<String>,
    pub film_stock_iso: Option<i32>,
    pub film_stock_type: Option<FilmStockType>,
    pub camera_brand: Option<String>,
    pub camera_model: Option<String>,
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
    pub dev_date: Option<String>,
    pub created_at: String,
}

const LIST_SELF_DEVS_SQL: &str = "\
    SELECT \
        ds.id AS dev_id, \
        r.id AS roll_pk, \
        r.roll_id, \
        r.status AS roll_status, \
        fs.brand AS film_stock_brand, \
        fs.name AS film_stock_name, \
        fs.iso AS film_stock_iso, \
        fs.stock_type AS film_stock_type, \
        c.brand AS camera_brand, \
        c.model AS camera_model, \
        ds.date_processed, \
        ds.developer, \
        ds.developer_dilution, \
        ds.fixer, \
        ds.fixer_dilution, \
        ds.stop_bath, \
        ds.wetting_agent, \
        ds.clearing_agent, \
        ds.temperature, \
        ds.agitation_notes, \
        ds.notes, \
        COALESCE(ds.date_processed, ds.created_at) AS dev_date, \
        ds.created_at \
    FROM development_selves ds \
    JOIN rolls r ON ds.roll_id = r.id \
    LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id \
    LEFT JOIN cameras c ON r.camera_id = c.id \
    ORDER BY dev_date DESC, ds.created_at DESC";

pub struct DevelopmentService;

/// Input for creating/replacing dev stages (used by set_stages).
pub struct StageInput {
    pub stage_name: String,
    pub duration_seconds: Option<i32>,
    pub notes: Option<String>,
    pub sort_order: i32,
}

impl DevelopmentService {
    // --- Lab Development ---

    pub async fn get_lab_dev_for_roll(
        db: &DatabaseConnection,
        roll_id: i32,
    ) -> Result<Option<development_lab::Model>, DbErr> {
        DevelopmentLab::find()
            .filter(development_lab::Column::RollId.eq(roll_id))
            .one(db)
            .await
    }

    pub async fn get_lab_dev_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<development_lab::Model>, DbErr> {
        DevelopmentLab::find_by_id(id).one(db).await
    }

    pub async fn create_lab_dev(
        db: &DatabaseConnection,
        model: development_lab::ActiveModel,
    ) -> Result<development_lab::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update_lab_dev(
        db: &impl ConnectionTrait,
        model: development_lab::ActiveModel,
    ) -> Result<development_lab::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete_lab_dev(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        DevelopmentLab::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    // --- Self Development ---

    pub async fn get_self_dev_for_roll(
        db: &DatabaseConnection,
        roll_id: i32,
    ) -> Result<Option<development_self::Model>, DbErr> {
        DevelopmentSelf::find()
            .filter(development_self::Column::RollId.eq(roll_id))
            .one(db)
            .await
    }

    pub async fn get_self_dev_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<development_self::Model>, DbErr> {
        DevelopmentSelf::find_by_id(id).one(db).await
    }

    pub async fn create_self_dev(
        db: &impl ConnectionTrait,
        model: development_self::ActiveModel,
    ) -> Result<development_self::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update_self_dev(
        db: &impl ConnectionTrait,
        model: development_self::ActiveModel,
    ) -> Result<development_self::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete_self_dev(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        // Dev stages are cascade-deleted by FK constraint
        DevelopmentSelf::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    // --- Dev Stages ---

    pub async fn list_stages(
        db: &DatabaseConnection,
        development_self_id: i32,
    ) -> Result<Vec<dev_stage::Model>, DbErr> {
        DevStage::find()
            .filter(dev_stage::Column::DevelopmentSelfId.eq(development_self_id))
            .order_by_asc(dev_stage::Column::SortOrder)
            .all(db)
            .await
    }

    /// Replace all stages for a self-development record (delete-all-then-insert).
    pub async fn set_stages(
        db: &impl ConnectionTrait,
        development_self_id: i32,
        stages: Vec<StageInput>,
    ) -> Result<(), DbErr> {
        // Delete existing stages
        DevStage::delete_many()
            .filter(dev_stage::Column::DevelopmentSelfId.eq(development_self_id))
            .exec(db)
            .await?;

        // Bulk insert new stages
        if !stages.is_empty() {
            let models: Vec<dev_stage::ActiveModel> = stages
                .into_iter()
                .map(|stage| dev_stage::ActiveModel {
                    development_self_id: Set(development_self_id),
                    stage_name: Set(stage.stage_name),
                    duration_seconds: Set(stage.duration_seconds),
                    notes: Set(stage.notes),
                    sort_order: Set(stage.sort_order),
                    ..Default::default()
                })
                .collect();
            DevStage::insert_many(models).exec(db).await?;
        }
        Ok(())
    }

    // --- List all self-developments (with joined context) ---

    pub async fn list_all_self_devs(
        db: &DatabaseConnection,
    ) -> Result<Vec<SelfDevListItem>, DbErr> {
        SelfDevListItem::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            LIST_SELF_DEVS_SQL.to_string(),
        ))
        .all(db)
        .await
    }

    pub async fn list_stages_for_dev_ids(
        db: &DatabaseConnection,
        ids: Vec<i32>,
    ) -> Result<Vec<dev_stage::Model>, DbErr> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        DevStage::find()
            .filter(dev_stage::Column::DevelopmentSelfId.is_in(ids))
            .order_by_asc(dev_stage::Column::DevelopmentSelfId)
            .order_by_asc(dev_stage::Column::SortOrder)
            .all(db)
            .await
    }
}
