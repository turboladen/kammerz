use sea_orm::*;
use serde::Serialize;

use crate::activity::{ActivitySignals, RollActivity, derive};
use crate::patch::now_string;
use crate::services::roll_event_service::RollEventService;
use crate::services::shot_service::ShotService;
use ::entity::roll::{self, Entity as Roll, PushPull};
use ::entity::roll_event::RollEventType;
use ::entity::{development_lab, development_self, shot};

/// Flat struct for rolls joined with camera and film stock data.
/// Mirrors the frontend's `RollWithDetails` TypeScript interface.
///
/// There is no stored `status` (ADR-0013): the lifecycle is derived from date
/// presence by [`crate::activity`]. The `self_dev_*` / `lab_dev_id` /
/// `negatives_date_received` join columns are the derivation's dev signals; the
/// handler wraps this row in a `RollView` that flattens in the derived
/// `activities`/`badge`/`group_key`/`done` fields.
#[derive(Debug, Serialize, FromQueryResult)]
pub struct RollWithDetails {
    // Roll fields
    pub id: i32,
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
    pub archive_na: bool,
    pub archive_na_reason: Option<String>,
    pub push_pull: Option<PushPull>,
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
    // Aggregate: number of shots logged on this roll (for the frame counter)
    pub shot_count: i64,
    // Negatives-pickup ingredients (LEFT JOINed from the roll's lab dev + its lab).
    // All Option because a roll may have no lab dev. `negatives_deadline` is
    // date_received + retention (default 30), computed in SQL; the frontend
    // derives the live countdown/state from it. Dates are always full `YYYY-MM-DD`
    // (validate_date_opt rejects partials — ADR-0011), but the query keeps a
    // `length >= 10` guard as defense in depth: SQLite's date() yields garbage/NULL
    // for a bare `YYYY`/`YYYY-MM`, so a legacy partial shows no countdown, not a bogus one.
    pub lab_dev_id: Option<i32>,
    pub lab_name: Option<String>,
    pub negatives_date_received: Option<String>,
    pub negatives_deadline: Option<String>,
    pub date_negatives_picked_up: Option<String>,
    pub negatives_not_collecting: Option<bool>,
    // Self-dev signals for activity derivation (internal — not part of the wire
    // contract). `negatives_date_received` doubles as the lab dev's completion date.
    #[serde(skip)]
    pub self_dev_id: Option<i32>,
    #[serde(skip)]
    pub self_dev_date_processed: Option<String>,
}

impl RollWithDetails {
    /// The activity-derivation signals for this roll (ADR-0013).
    pub fn signals(&self) -> ActivitySignals {
        ActivitySignals {
            shot_count: self.shot_count,
            date_loaded: self.date_loaded.clone(),
            date_finished: self.date_finished.clone(),
            scan_started: self.scan_started.clone(),
            date_scanned: self.date_scanned.clone(),
            post_processing_started: self.post_processing_started.clone(),
            date_post_processed: self.date_post_processed.clone(),
            date_archived: self.date_archived.clone(),
            archive_na: self.archive_na,
            ..Default::default()
        }
        .with_dev(
            self.lab_dev_id,
            self.negatives_date_received.clone(),
            self.self_dev_id,
            self.self_dev_date_processed.clone(),
        )
    }

    /// The derived activity view (per-activity states, badge, group key).
    pub fn activity(&self) -> RollActivity {
        derive(&self.signals())
    }
}

const ROLLS_WITH_DETAILS_SQL: &str = "\
    SELECT r.id, r.roll_id, r.camera_id, r.film_stock_id, r.lens_id, \
           r.frame_count, r.date_loaded, r.date_finished, \
           r.scan_started, r.date_scanned, r.post_processing_started, \
           r.date_post_processed, r.date_archived, \
           r.archive_location, r.archive_na, r.archive_na_reason, \
           r.push_pull, r.notes, r.created_at, r.updated_at, \
           c.brand AS camera_brand, c.model AS camera_model, \
           fs.brand AS film_stock_brand, fs.name AS film_stock_name, \
           fs.iso AS film_stock_iso, \
           l.brand AS lens_brand, \
           COALESCE(l.model, l.focal_length) AS lens_name, \
           (SELECT COUNT(*) FROM shots s WHERE s.roll_id = r.id) AS shot_count, \
           dl.id AS lab_dev_id, \
           lab.name AS lab_name, \
           dl.date_received AS negatives_date_received, \
           CASE WHEN dl.date_received IS NOT NULL AND length(dl.date_received) >= 10 \
                THEN date(dl.date_received, '+' || COALESCE(lab.negative_retention_days, 30) || ' days') \
                ELSE NULL END AS negatives_deadline, \
           dl.date_negatives_picked_up AS date_negatives_picked_up, \
           dl.negatives_not_collecting AS negatives_not_collecting, \
           ds.id AS self_dev_id, \
           ds.date_processed AS self_dev_date_processed \
    FROM rolls r \
    LEFT JOIN cameras c ON r.camera_id = c.id \
    LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id \
    LEFT JOIN lenses l ON r.lens_id = l.id \
    LEFT JOIN development_labs dl ON dl.roll_id = r.id \
    LEFT JOIN labs lab ON dl.lab_id = lab.id \
    LEFT JOIN development_selves ds ON ds.roll_id = r.id";

/// A development record to synthesize during import so a dev-stage legacy status
/// derives its intended activity state (kammerz-gsj6). Lab vs self mirrors the
/// legacy status the paper note used; the completion date is an honest
/// lower-bound borrow (never fabricated) and is `None` when nothing recorded
/// exists to borrow — in which case the roll derives "Developing" rather than
/// done. Terminal statuses create no record (development derives implicitly-done
/// from a tail date — see `activity.rs`).
#[derive(Debug, PartialEq)]
pub enum ImportDevRecord {
    Lab { date_received: Option<String> },
    SelfDev { date_processed: Option<String> },
}

/// Data for a single shot during roll import.
pub struct ImportShotEntry {
    pub frame_number: String,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub lens_ids: Option<Vec<i32>>,
}

pub struct RollService;

impl RollService {
    pub async fn list_all_with_details(
        db: &DatabaseConnection,
    ) -> Result<Vec<RollWithDetails>, DbErr> {
        let sql = format!("{ROLLS_WITH_DETAILS_SQL} ORDER BY r.created_at DESC");
        RollWithDetails::find_by_statement(Statement::from_string(db.get_database_backend(), sql))
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
        db: &impl ConnectionTrait,
        model: roll::ActiveModel,
    ) -> Result<roll::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &impl ConnectionTrait,
        model: roll::ActiveModel,
    ) -> Result<roll::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        let res = Roll::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound(format!("Roll {id} not found")));
        }
        Ok(())
    }

    pub async fn list_for_camera(
        db: &DatabaseConnection,
        camera_id: i32,
    ) -> Result<Vec<RollWithDetails>, DbErr> {
        let sql =
            format!("{ROLLS_WITH_DETAILS_SQL} WHERE r.camera_id = $1 ORDER BY r.created_at DESC");
        RollWithDetails::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [camera_id.into()],
        ))
        .all(db)
        .await
    }

    /// Import a roll with its shots in a single transaction. An optional
    /// [`ImportDevRecord`] (for dev-stage legacy statuses) is inserted in the same
    /// transaction so the imported roll derives the right activity state
    /// (kammerz-gsj6).
    pub async fn import_roll(
        db: &DatabaseConnection,
        roll_model: roll::ActiveModel,
        shot_entries: Vec<ImportShotEntry>,
        dev: Option<ImportDevRecord>,
    ) -> Result<i32, TransactionError<DbErr>> {
        db.transaction::<_, i32, DbErr>(|txn| {
            Box::pin(async move {
                let roll_result = roll_model.insert(txn).await?;
                let new_roll_id = roll_result.id;

                // Bulk import emits only the founding roll_loaded event — per-shot
                // events are intentionally NOT logged, so an imported timeline has a
                // sensible start without dozens of shot entries.
                RollEventService::record(
                    txn,
                    new_roll_id,
                    RollEventType::RollLoaded,
                    None,
                    None,
                    "Roll loaded".to_string(),
                )
                .await?;

                let now = now_string();

                // Synthesize the dev record for a dev-stage import inside the same
                // transaction (rest defaulted, mirroring routes/development.rs — the
                // DB default handles negatives_not_collecting).
                match dev {
                    Some(ImportDevRecord::Lab { date_received }) => {
                        development_lab::ActiveModel {
                            roll_id: Set(new_roll_id),
                            date_received: Set(date_received),
                            created_at: Set(now.clone()),
                            updated_at: Set(now.clone()),
                            ..Default::default()
                        }
                        .insert(txn)
                        .await?;
                    }
                    Some(ImportDevRecord::SelfDev { date_processed }) => {
                        development_self::ActiveModel {
                            roll_id: Set(new_roll_id),
                            date_processed: Set(date_processed),
                            created_at: Set(now.clone()),
                            updated_at: Set(now.clone()),
                            ..Default::default()
                        }
                        .insert(txn)
                        .await?;
                    }
                    None => {}
                }

                for entry in shot_entries {
                    let shot_model = shot::ActiveModel {
                        roll_id: Set(new_roll_id),
                        frame_number: Set(entry.frame_number),
                        aperture: Set(entry.aperture),
                        shutter_speed: Set(entry.shutter_speed),
                        date: Set(entry.date),
                        time: Set(entry.time),
                        location: Set(entry.location),
                        notes: Set(entry.notes),
                        created_at: Set(now.clone()),
                        updated_at: Set(now.clone()),
                        ..Default::default()
                    };
                    let shot_result = shot_model.insert(txn).await?;

                    if let Some(lens_ids) = entry.lens_ids {
                        if !lens_ids.is_empty() {
                            ShotService::set_lenses_for_shot(txn, shot_result.id, lens_ids).await?;
                        }
                    }
                }

                Ok(new_roll_id)
            })
        })
        .await
    }

    /// Suggest a roll ID in YYMMDD-N format.
    pub async fn suggest_id(db: &DatabaseConnection) -> Result<String, DbErr> {
        let now = chrono::Local::now();
        let prefix = now.format("%y%m%d").to_string();

        #[derive(Debug, FromQueryResult)]
        struct MaxRow {
            max_suffix: i64,
        }

        // Derive the next suffix from the largest existing one, not a row count:
        // deleting a non-last same-day roll shrinks the count, so a count-based
        // suffix would re-suggest a surviving roll's id and fail the UNIQUE
        // constraint on roll_id (kammerz-cg1). substr strips the "YYMMDD-"
        // prefix; SQLite CAST reads a leading numeric prefix and ignores the
        // rest ("2-retry" -> 2), and a tail with no leading digits CASTs to 0 —
        // both harmless here.
        let stub = format!("{prefix}-");
        let pattern = format!("{prefix}-%");
        let row = MaxRow::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT COALESCE(MAX(CAST(substr(roll_id, length($1) + 1) AS INTEGER)), 0) \
             AS max_suffix FROM rolls WHERE roll_id LIKE $2",
            [stub.into(), pattern.into()],
        ))
        .one(db)
        .await?;

        let next = row.map(|r| r.max_suffix).unwrap_or(0) + 1;
        Ok(format!("{prefix}-{next}"))
    }
}
