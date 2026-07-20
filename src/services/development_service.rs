use sea_orm::*;
use serde::Serialize;

use ::entity::dev_stage::{self, Entity as DevStage};
use ::entity::development_lab::{self, Entity as DevelopmentLab};
use ::entity::development_self::{self, Entity as DevelopmentSelf};
use ::entity::film_stock::FilmStockType;

use crate::activity::{ActivitySignals, derive};

/// The server-derived activity summary (`badge`, `group_key`) for a dev-list row
/// (ADR-0013), so the developments UI renders the same phase Badge as every other
/// roll list. A dev record always exists here, so the roll is at least in
/// development; the derivation picks the exact phase from the tail dates.
fn dev_roll_activity(
    is_lab_dev: bool,
    dev_completion: Option<String>,
    roll_tail: ActivitySignals,
) -> (String, i32) {
    // `roll_tail` carries every derivation-relevant roll column (the *_started
    // fields and archive_na were once defaulted, making this list's badge/
    // group_key disagree with the canonical roll list — e.g. a mid-scan roll
    // read "To scan" here and "Scanning" there, and an archive-N/A roll read
    // "To archive" instead of "Done"). Shooting's shot_count/date_loaded/
    // date_finished stay defaulted: a dev record exists on every row, so
    // shooting is implicitly done and those fields cannot affect the output.
    let activity = derive(&ActivitySignals {
        has_dev: true,
        is_lab_dev,
        dev_completion,
        ..roll_tail
    });
    (activity.badge, activity.group_key)
}

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
    // Server-derived activity summary, computed in Rust after the query
    // (placeholders in SQL). Renders the roll's phase Badge (ADR-0013).
    pub badge: String,
    pub group_key: i32,
    // Roll tail dates for the derivation — not part of the wire contract.
    #[serde(skip)]
    pub roll_date_scanned: Option<String>,
    #[serde(skip)]
    pub roll_date_post_processed: Option<String>,
    #[serde(skip)]
    pub roll_date_archived: Option<String>,
    #[serde(skip)]
    pub roll_scan_started: Option<String>,
    #[serde(skip)]
    pub roll_post_processing_started: Option<String>,
    #[serde(skip)]
    pub roll_archive_na: bool,
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

/// Flat struct for lab-developments joined with roll, film stock, camera, and
/// lab data. Lab devs have no stages, so unlike [`SelfDevListItem`] this is
/// returned directly (no stage merge).
#[derive(Debug, Serialize, FromQueryResult)]
pub struct LabDevListItem {
    pub dev_id: i32,
    pub roll_pk: i32,
    pub roll_id: String,
    // Server-derived activity summary, computed in Rust after the query
    // (placeholders in SQL). Renders the roll's phase Badge (ADR-0013).
    pub badge: String,
    pub group_key: i32,
    // Roll tail dates for the derivation — not part of the wire contract.
    #[serde(skip)]
    pub roll_date_scanned: Option<String>,
    #[serde(skip)]
    pub roll_date_post_processed: Option<String>,
    #[serde(skip)]
    pub roll_date_archived: Option<String>,
    #[serde(skip)]
    pub roll_scan_started: Option<String>,
    #[serde(skip)]
    pub roll_post_processing_started: Option<String>,
    #[serde(skip)]
    pub roll_archive_na: bool,
    pub film_stock_brand: Option<String>,
    pub film_stock_name: Option<String>,
    pub film_stock_iso: Option<i32>,
    pub film_stock_type: Option<FilmStockType>,
    pub camera_brand: Option<String>,
    pub camera_model: Option<String>,
    pub lab_name: Option<String>,
    pub date_dropped_off: Option<String>,
    pub date_received: Option<String>,
    pub cost: Option<f64>,
    pub notes: Option<String>,
    pub dev_date: Option<String>,
    pub created_at: String,
}

const LIST_LAB_DEVS_SQL: &str = "\
    SELECT \
        dl.id AS dev_id, \
        r.id AS roll_pk, \
        r.roll_id, \
        '' AS badge, \
        0 AS group_key, \
        r.date_scanned AS roll_date_scanned, \
        r.date_post_processed AS roll_date_post_processed, \
        r.date_archived AS roll_date_archived, \
        r.scan_started AS roll_scan_started, \
        r.post_processing_started AS roll_post_processing_started, \
        r.archive_na AS roll_archive_na, \
        fs.brand AS film_stock_brand, \
        fs.name AS film_stock_name, \
        fs.iso AS film_stock_iso, \
        fs.stock_type AS film_stock_type, \
        c.brand AS camera_brand, \
        c.model AS camera_model, \
        l.name AS lab_name, \
        dl.date_dropped_off, \
        dl.date_received, \
        dl.cost, \
        dl.notes, \
        COALESCE(dl.date_received, dl.date_dropped_off, dl.created_at) AS dev_date, \
        dl.created_at \
    FROM development_labs dl \
    JOIN rolls r ON dl.roll_id = r.id \
    LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id \
    LEFT JOIN cameras c ON r.camera_id = c.id \
    LEFT JOIN labs l ON dl.lab_id = l.id \
    ORDER BY dev_date DESC, dl.created_at DESC";

const LIST_SELF_DEVS_SQL: &str = "\
    SELECT \
        ds.id AS dev_id, \
        r.id AS roll_pk, \
        r.roll_id, \
        '' AS badge, \
        0 AS group_key, \
        r.date_scanned AS roll_date_scanned, \
        r.date_post_processed AS roll_date_post_processed, \
        r.date_archived AS roll_date_archived, \
        r.scan_started AS roll_scan_started, \
        r.post_processing_started AS roll_post_processing_started, \
        r.archive_na AS roll_archive_na, \
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

    /// List every lab-development with its joined roll, film stock, camera, and
    /// lab context. Lab devs have no stages, so this returns the flat rows
    /// directly (no batch-merge step like the self-dev list).
    pub async fn list_all_lab_devs(db: &DatabaseConnection) -> Result<Vec<LabDevListItem>, DbErr> {
        let mut items = LabDevListItem::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            LIST_LAB_DEVS_SQL.to_string(),
        ))
        .all(db)
        .await?;
        for item in &mut items {
            (item.badge, item.group_key) = dev_roll_activity(
                true,
                item.date_received.clone(),
                ActivitySignals {
                    scan_started: item.roll_scan_started.clone(),
                    date_scanned: item.roll_date_scanned.clone(),
                    post_processing_started: item.roll_post_processing_started.clone(),
                    date_post_processed: item.roll_date_post_processed.clone(),
                    date_archived: item.roll_date_archived.clone(),
                    archive_na: item.roll_archive_na,
                    ..Default::default()
                },
            );
        }
        Ok(items)
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
        let mut items = SelfDevListItem::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            LIST_SELF_DEVS_SQL.to_string(),
        ))
        .all(db)
        .await?;
        for item in &mut items {
            (item.badge, item.group_key) = dev_roll_activity(
                false,
                item.date_processed.clone(),
                ActivitySignals {
                    scan_started: item.roll_scan_started.clone(),
                    date_scanned: item.roll_date_scanned.clone(),
                    post_processing_started: item.roll_post_processing_started.clone(),
                    date_post_processed: item.roll_date_post_processed.clone(),
                    date_archived: item.roll_date_archived.clone(),
                    archive_na: item.roll_archive_na,
                    ..Default::default()
                },
            );
        }
        Ok(items)
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
