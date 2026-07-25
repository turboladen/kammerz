//! Activity-based roll lifecycle (ADR-0013): add the columns the five activities
//! need, backfill dates from the legacy `status` (recorded dates only, never
//! fabricated), then drop `status` and its index.
//!
//! Idempotency: `execute_unprepared` auto-commits per statement, so every step is
//! individually guarded — column adds check `PRAGMA table_info`, the backfill
//! UPDATEs only fill currently-NULL targets, and the index/column drops use
//! `IF EXISTS` / a presence check. A re-run after a partial failure is a no-op.
//!
//! Test seam (kammerz-9fx pattern): [`BACKFILL_ORDER`] + the pure
//! [`backfilled_dates`] fn are `pub` and re-exported from the crate root, so
//! `tests/` exercises the exact mapping this migration applies (and `import.rs`
//! reuses it to derive dates for imported rolls). The mapping is unit-tested
//! directly rather than through a fresh-DB migration-at-init run, because that
//! run has no legacy `status` rows to backfill (and the column is dropped here).

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, Statement};

/// The legacy roll-status vocabulary, in progression order. This is an INPUT
/// parse type, not app state (the stored `RollStatus` was retired, ADR-0013):
/// it names what paper shot-logs say so the import backfill can translate them
/// into dates. `Ord` follows declaration order — "reached at least milestone X"
/// comparisons are plain `>=`. The kebab-case wire strings (`at-lab`,
/// `post-processed`, …) come from strum and are pinned by a unit test: they are
/// the import API contract and must never drift.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, strum::EnumString, strum::VariantNames,
)]
#[strum(serialize_all = "kebab-case")]
pub enum LegacyStatus {
    Loaded,
    Shooting,
    Shot,
    AtLab,
    LabDone,
    Developing,
    Developed,
    Scanned,
    PostProcessed,
    Archived,
}

/// The wire strings in progression order, derived FROM the enum (one source).
// NOTE: the import page's statusOptions dropdown
// (frontend/src/routes/(app)/import/+page.svelte) must stay a subset of this
// list — routes/import.rs 422s anything outside it. Update both together.
pub const BACKFILL_ORDER: &[&str] = <LegacyStatus as strum::VariantNames>::VARIANTS;

/// The dates the backfill would set for a roll, given its legacy status and the
/// dates available to borrow. A field is `Some` only when the roll reached the
/// milestone that gates it, the target column is currently NULL, and a real
/// recorded date exists to borrow — never a fabricated date.
#[derive(Debug, Default, PartialEq)]
pub struct BackfilledDates {
    pub date_finished: Option<String>,
    pub date_scanned: Option<String>,
    pub date_archived: Option<String>,
}

/// Pure backfill mapping (spec table). Shared by this migration's `up` and by
/// `import.rs` so imported rolls derive the same way. `max_shot_date` is
/// `MAX(shots.date)` for the roll; `dev_completion` is the lab `date_received` or
/// self `date_processed`.
// Genuine false positive: the eight parameters ARE the spec table's columns
// (one status + seven date signals); bundling them into a struct would only
// move the same list behind a constructor at every call site.
#[allow(clippy::too_many_arguments)]
pub fn backfilled_dates(
    status: LegacyStatus,
    date_loaded: Option<&str>,
    date_finished: Option<&str>,
    max_shot_date: Option<&str>,
    dev_completion: Option<&str>,
    date_scanned: Option<&str>,
    date_post_processed: Option<&str>,
    date_archived: Option<&str>,
) -> BackfilledDates {
    let mut out = BackfilledDates::default();

    // status >= shot: date_finished := max(shot dates) ?? date_loaded
    if status >= LegacyStatus::Shot && date_finished.is_none() {
        out.date_finished = max_shot_date.or(date_loaded).map(str::to_string);
    }

    // status >= scanned: date_scanned := date_post_processed ?? dev completion
    if status >= LegacyStatus::Scanned && date_scanned.is_none() {
        out.date_scanned = date_post_processed.or(dev_completion).map(str::to_string);
    }

    // status >= archived: date_archived := date_post_processed ?? date_scanned
    // (existing or the value we just chose above) ?? dev completion
    if status >= LegacyStatus::Archived && date_archived.is_none() {
        let effective_scanned = date_scanned
            .map(str::to_string)
            .or(out.date_scanned.clone());
        out.date_archived = date_post_processed
            .map(str::to_string)
            .or(effective_scanned)
            .or(dev_completion.map(str::to_string));
    }

    out
}

#[derive(Iden)]
enum Rolls {
    Table,
    ScanStarted,
    PostProcessingStarted,
    ArchiveLocation,
    ArchiveNa,
    ArchiveNaReason,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Column names currently on `rolls`, via `PRAGMA table_info`. Used to guard the
/// otherwise-unrepeatable ADD/DROP COLUMN statements (SQLite has no `IF [NOT]
/// EXISTS` for columns).
async fn roll_columns(db: &impl ConnectionTrait) -> Result<Vec<String>, DbErr> {
    let rows = db
        .query_all(Statement::from_string(
            db.get_database_backend(),
            "PRAGMA table_info(rolls)".to_owned(),
        ))
        .await?;
    rows.into_iter()
        .map(|row| row.try_get("", "name"))
        .collect()
}

/// Add one column to `rolls` only if it is not already present (SQLite lacks
/// `ADD COLUMN IF NOT EXISTS`), keeping the step idempotent.
async fn add_col_if_missing(
    manager: &SchemaManager<'_>,
    existing: &[String],
    name: &str,
    mut def: ColumnDef,
) -> Result<(), DbErr> {
    if existing.iter().any(|c| c == name) {
        return Ok(());
    }
    manager
        .alter_table(
            Table::alter()
                .table(Rolls::Table)
                .add_column(&mut def)
                .to_owned(),
        )
        .await
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // --- 1. Add the new columns (guarded; SQLite: one ADD COLUMN per ALTER) ---
        let existing = roll_columns(db).await?;
        add_col_if_missing(
            manager,
            &existing,
            "scan_started",
            ColumnDef::new(Rolls::ScanStarted).text().null().to_owned(),
        )
        .await?;
        add_col_if_missing(
            manager,
            &existing,
            "post_processing_started",
            ColumnDef::new(Rolls::PostProcessingStarted)
                .text()
                .null()
                .to_owned(),
        )
        .await?;
        add_col_if_missing(
            manager,
            &existing,
            "archive_location",
            ColumnDef::new(Rolls::ArchiveLocation)
                .text()
                .null()
                .to_owned(),
        )
        .await?;
        add_col_if_missing(
            manager,
            &existing,
            "archive_na",
            // Integer default 0 (never a &str — a string default emits a quoted
            // literal per the healthie-audit finding).
            ColumnDef::new(Rolls::ArchiveNa)
                .boolean()
                .not_null()
                .default(0)
                .to_owned(),
        )
        .await?;
        add_col_if_missing(
            manager,
            &existing,
            "archive_na_reason",
            ColumnDef::new(Rolls::ArchiveNaReason)
                .text()
                .null()
                .to_owned(),
        )
        .await?;

        // --- 2. Backfill dates from legacy status (only if the column still exists) ---
        if roll_columns(db).await?.iter().any(|c| c == "status") {
            let rows = db
                .query_all(Statement::from_string(
                    db.get_database_backend(),
                    "SELECT r.id, r.status, r.date_loaded, r.date_finished, \
                            r.date_scanned, r.date_post_processed, r.date_archived, \
                            (SELECT MAX(s.date) FROM shots s WHERE s.roll_id = r.id) AS max_shot_date, \
                            COALESCE( \
                                (SELECT dl.date_received FROM development_labs dl WHERE dl.roll_id = r.id LIMIT 1), \
                                (SELECT ds.date_processed FROM development_selves ds WHERE ds.roll_id = r.id LIMIT 1) \
                            ) AS dev_completion \
                     FROM rolls r"
                        .to_owned(),
                ))
                .await?;

            for row in rows {
                let id: i32 = row.try_get("", "id")?;
                let status: String = row.try_get("", "status")?;
                let date_loaded: Option<String> = row.try_get("", "date_loaded")?;
                let date_finished: Option<String> = row.try_get("", "date_finished")?;
                let date_scanned: Option<String> = row.try_get("", "date_scanned")?;
                let date_post_processed: Option<String> = row.try_get("", "date_post_processed")?;
                let date_archived: Option<String> = row.try_get("", "date_archived")?;
                let max_shot_date: Option<String> = row.try_get("", "max_shot_date")?;
                let dev_completion: Option<String> = row.try_get("", "dev_completion")?;

                // Historical rows can hold arbitrary strings; an unparseable
                // status backfills nothing (the row is left untouched).
                let filled = status
                    .parse::<LegacyStatus>()
                    .map(|legacy| {
                        backfilled_dates(
                            legacy,
                            date_loaded.as_deref(),
                            date_finished.as_deref(),
                            max_shot_date.as_deref(),
                            dev_completion.as_deref(),
                            date_scanned.as_deref(),
                            date_post_processed.as_deref(),
                            date_archived.as_deref(),
                        )
                    })
                    .unwrap_or_default();

                // Each UPDATE re-guards on NULL so the step is idempotent even if the
                // pure fn's precondition and the row drift between a partial re-run.
                for (column, value) in [
                    ("date_finished", filled.date_finished),
                    ("date_scanned", filled.date_scanned),
                    ("date_archived", filled.date_archived),
                ] {
                    if let Some(v) = value {
                        db.execute(Statement::from_sql_and_values(
                            db.get_database_backend(),
                            format!(
                                "UPDATE rolls SET {column} = $1 WHERE id = $2 AND {column} IS NULL"
                            ),
                            [v.into(), id.into()],
                        ))
                        .await?;
                    }
                }
            }
        }

        // --- 3. Drop the status index, then the column (index blocks DROP COLUMN) ---
        db.execute_unprepared("DROP INDEX IF EXISTS idx_rolls_status")
            .await?;
        if roll_columns(db).await?.iter().any(|c| c == "status") {
            manager
                .alter_table(
                    Table::alter()
                        .table(Rolls::Table)
                        .drop_column(Alias::new("status"))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Intentional no-op: the derived activity model supersedes `status`, and
        // the backfill borrowed real dates (not reversible to the exact prior
        // enum). Re-adding a `status` column would have no correct value to fill.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Migrator;
    use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection};

    #[test]
    fn loaded_and_shooting_get_no_dates() {
        for status in [LegacyStatus::Loaded, LegacyStatus::Shooting] {
            let out = backfilled_dates(
                status,
                Some("2026-01-01"),
                None,
                Some("2026-01-03"),
                None,
                None,
                None,
                None,
            );
            assert_eq!(out, BackfilledDates::default(), "{status:?}");
        }
    }

    #[test]
    fn shot_backfills_date_finished_from_max_shot_date() {
        let out = backfilled_dates(
            LegacyStatus::Shot,
            Some("2026-01-01"),
            None,
            Some("2026-01-05"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(out.date_finished.as_deref(), Some("2026-01-05"));
        assert_eq!(out.date_scanned, None);
        assert_eq!(out.date_archived, None);
    }

    #[test]
    fn date_finished_falls_back_to_date_loaded() {
        let out = backfilled_dates(
            LegacyStatus::Shot,
            Some("2026-01-01"),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(out.date_finished.as_deref(), Some("2026-01-01"));
    }

    #[test]
    fn dev_stage_statuses_backfill_finished_only() {
        for status in [
            LegacyStatus::AtLab,
            LegacyStatus::LabDone,
            LegacyStatus::Developing,
            LegacyStatus::Developed,
        ] {
            let out = backfilled_dates(
                status,
                Some("2026-01-01"),
                None,
                Some("2026-01-05"),
                Some("2026-01-10"),
                None,
                None,
                None,
            );
            assert_eq!(
                out.date_finished.as_deref(),
                Some("2026-01-05"),
                "{status:?}"
            );
            assert_eq!(out.date_scanned, None, "{status:?}");
            assert_eq!(out.date_archived, None, "{status:?}");
        }
    }

    #[test]
    fn scanned_backfills_from_post_processed_then_dev_completion() {
        let from_pp = backfilled_dates(
            LegacyStatus::Scanned,
            Some("2026-01-01"),
            Some("2026-01-05"),
            None,
            Some("2026-01-10"),
            None,
            Some("2026-01-14"),
            None,
        );
        assert_eq!(from_pp.date_scanned.as_deref(), Some("2026-01-14"));

        let from_dev = backfilled_dates(
            LegacyStatus::Scanned,
            Some("2026-01-01"),
            Some("2026-01-05"),
            None,
            Some("2026-01-10"),
            None,
            None,
            None,
        );
        assert_eq!(from_dev.date_scanned.as_deref(), Some("2026-01-10"));
    }

    #[test]
    fn archived_borrows_the_backfilled_scanned_date() {
        // No post-processed, no existing scanned: date_archived chains off the
        // date_scanned this same call derives from dev completion.
        let out = backfilled_dates(
            LegacyStatus::Archived,
            Some("2026-01-01"),
            Some("2026-01-05"),
            None,
            Some("2026-01-10"),
            None,
            None,
            None,
        );
        assert_eq!(out.date_scanned.as_deref(), Some("2026-01-10"));
        assert_eq!(out.date_archived.as_deref(), Some("2026-01-10"));
    }

    #[test]
    fn archived_prefers_post_processed() {
        let out = backfilled_dates(
            LegacyStatus::Archived,
            Some("2026-01-01"),
            Some("2026-01-05"),
            None,
            Some("2026-01-10"),
            Some("2026-01-12"),
            Some("2026-01-14"),
            None,
        );
        assert_eq!(out.date_archived.as_deref(), Some("2026-01-14"));
    }

    #[test]
    fn already_populated_targets_are_left_untouched() {
        let out = backfilled_dates(
            LegacyStatus::Archived,
            Some("2026-01-01"),
            Some("2026-01-05"),
            Some("2026-01-04"),
            Some("2026-01-10"),
            Some("2026-01-12"),
            Some("2026-01-14"),
            Some("2026-01-20"),
        );
        assert_eq!(out, BackfilledDates::default());
    }

    #[test]
    fn no_dates_to_borrow_yields_nothing() {
        // An imported archived roll with no shots and no dev record degrades
        // (kammerz-gsj6): nothing to borrow, so no dates are fabricated.
        let out = backfilled_dates(
            LegacyStatus::Archived,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(out, BackfilledDates::default());
    }

    #[test]
    fn unknown_status_fails_to_parse_and_backfills_nothing() {
        // The old string API returned a silent no-op for unknown statuses; the
        // enum moves that to a parse failure (import 422s). The migration's up()
        // composes the failure into a zero-backfill exactly like this — assert
        // the COMPOSED shape so an edit to that branch can't silently change
        // what happens to historical rows with unrecognized statuses.
        assert!("bogus".parse::<LegacyStatus>().is_err());
        let filled = "bogus"
            .parse::<LegacyStatus>()
            .map(|legacy| {
                backfilled_dates(
                    legacy,
                    Some("2026-01-01"),
                    None,
                    Some("2026-01-05"),
                    None,
                    None,
                    None,
                    None,
                )
            })
            .unwrap_or_default();
        assert_eq!(filled, BackfilledDates::default());
    }

    #[test]
    fn wire_strings_are_pinned() {
        // The kebab-case strings ARE the import API contract (and the frontend
        // statusOptions vocabulary) — a variant rename must not drift them.
        assert_eq!(
            BACKFILL_ORDER,
            [
                "loaded",
                "shooting",
                "shot",
                "at-lab",
                "lab-done",
                "developing",
                "developed",
                "scanned",
                "post-processed",
                "archived",
            ]
        );
        for s in BACKFILL_ORDER {
            assert!(s.parse::<LegacyStatus>().is_ok(), "{s} must round-trip");
        }
    }

    // ── DB-level test of up()'s backfill against a legacy `status` schema ──
    //
    // The pure `backfilled_dates` mapping is unit-tested above, but up()'s actual
    // SQL (status-column detection, the dev-completion COALESCE subqueries,
    // MAX(shots.date), the guarded per-row UPDATEs) never runs in the normal
    // suite: every fresh test DB runs ALL migrations, and this one drops `status`.
    // Here we stop at m029 — the REAL pre-m030 schema, `status` still present —
    // seed legacy rows, run up(), and assert the resulting dates equal what
    // `backfilled_dates` computes for the same inputs, cross-checking the SQL path
    // against the pure mapping. (Migrator::up(Some(29)) rather than a hand-built
    // schema slice keeps the rolls/shots/dev tables exactly production's.)

    /// A fresh in-memory DB migrated through m029 only (this migration not yet
    /// applied), FK enforcement OFF exactly as `db.rs` sets it before migrating
    /// (so seeded child rows needn't reference real cameras/labs). A single
    /// connection keeps the in-memory DB alive across statements.
    async fn migrated_through_029() -> DatabaseConnection {
        let mut opt = ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1).min_connections(1);
        let db = Database::connect(opt).await.unwrap();
        db.execute_unprepared("PRAGMA foreign_keys=OFF")
            .await
            .unwrap();
        // Some(29): apply the first 29 migrations (through m029), none after — so
        // `rolls.status` still exists (this migration, #30, drops it).
        Migrator::up(&db, Some(29)).await.unwrap();
        db
    }

    /// A test-controlled date literal or NULL for a raw INSERT.
    fn sql_opt(v: Option<&str>) -> String {
        v.map_or_else(|| "NULL".to_string(), |s| format!("'{s}'"))
    }

    /// The three backfill-target columns for one roll after the migration ran.
    async fn roll_target_dates(
        db: &DatabaseConnection,
        id: i32,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let row = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                format!(
                    "SELECT date_finished, date_scanned, date_archived FROM rolls WHERE id = {id}"
                ),
            ))
            .await
            .unwrap()
            .unwrap();
        (
            row.try_get("", "date_finished").unwrap(),
            row.try_get("", "date_scanned").unwrap(),
            row.try_get("", "date_archived").unwrap(),
        )
    }

    /// One seeded legacy roll and the child rows that drive its backfill.
    struct Scenario {
        id: i32,
        roll_id: &'static str,
        status: &'static str,
        legacy: LegacyStatus,
        date_loaded: Option<&'static str>,
        date_finished: Option<&'static str>,
        date_scanned: Option<&'static str>,
        date_post_processed: Option<&'static str>,
        date_archived: Option<&'static str>,
        shot_dates: &'static [&'static str],
        lab_received: Option<&'static str>,
        self_processed: Option<&'static str>,
    }

    impl Scenario {
        /// `MAX(shots.date)` — ISO dates sort lexically, matching the SQL's MAX.
        fn max_shot_date(&self) -> Option<&'static str> {
            self.shot_dates.iter().max().copied()
        }

        /// The dev completion the SQL's COALESCE picks: lab `date_received` first,
        /// then self `date_processed`.
        fn dev_completion(&self) -> Option<&'static str> {
            self.lab_received.or(self.self_processed)
        }

        /// The pure mapping's verdict for this roll's seeded inputs — the oracle
        /// the SQL path must reproduce.
        fn expected(&self) -> BackfilledDates {
            backfilled_dates(
                self.legacy,
                self.date_loaded,
                self.date_finished,
                self.max_shot_date(),
                self.dev_completion(),
                self.date_scanned,
                self.date_post_processed,
                self.date_archived,
            )
        }

        async fn seed(&self, db: &DatabaseConnection) {
            db.execute_unprepared(&format!(
                "INSERT INTO rolls (id, roll_id, status, date_loaded, date_finished, \
                 date_scanned, date_post_processed, date_archived) \
                 VALUES ({}, '{}', '{}', {}, {}, {}, {}, {})",
                self.id,
                self.roll_id,
                self.status,
                sql_opt(self.date_loaded),
                sql_opt(self.date_finished),
                sql_opt(self.date_scanned),
                sql_opt(self.date_post_processed),
                sql_opt(self.date_archived),
            ))
            .await
            .unwrap();

            // Distinct frame_number per shot (UNIQUE(roll_id, frame_number)).
            for (i, date) in self.shot_dates.iter().enumerate() {
                db.execute_unprepared(&format!(
                    "INSERT INTO shots (roll_id, frame_number, date) VALUES ({}, '{}', '{date}')",
                    self.id, i,
                ))
                .await
                .unwrap();
            }
            if let Some(d) = self.lab_received {
                db.execute_unprepared(&format!(
                    "INSERT INTO development_labs (roll_id, date_received) VALUES ({}, '{d}')",
                    self.id,
                ))
                .await
                .unwrap();
            }
            if let Some(d) = self.self_processed {
                db.execute_unprepared(&format!(
                    "INSERT INTO development_selves (roll_id, date_processed) VALUES ({}, '{d}')",
                    self.id,
                ))
                .await
                .unwrap();
            }
        }

        /// The DB's post-migration dates must equal the pre-existing seeded value
        /// (the `WHERE ... IS NULL` guard leaves it) or, when that was NULL, the
        /// value `backfilled_dates` derived — i.e. `expected.or(seeded)`.
        async fn assert_backfilled(&self, db: &DatabaseConnection) {
            let expected = self.expected();
            let (df, ds, da) = roll_target_dates(db, self.id).await;
            assert_eq!(
                df.as_deref(),
                expected.date_finished.as_deref().or(self.date_finished),
                "date_finished for {}",
                self.roll_id
            );
            assert_eq!(
                ds.as_deref(),
                expected.date_scanned.as_deref().or(self.date_scanned),
                "date_scanned for {}",
                self.roll_id
            );
            assert_eq!(
                da.as_deref(),
                expected.date_archived.as_deref().or(self.date_archived),
                "date_archived for {}",
                self.roll_id
            );
        }
    }

    #[tokio::test]
    async fn up_backfills_dates_from_legacy_status() {
        let scenarios = [
            // 1. `shot` with shots → date_finished := MAX(shot dates).
            Scenario {
                id: 1,
                roll_id: "R-shot-shots",
                status: "shot",
                legacy: LegacyStatus::Shot,
                date_loaded: Some("2026-01-01"),
                date_finished: None,
                date_scanned: None,
                date_post_processed: None,
                date_archived: None,
                shot_dates: &["2026-01-03", "2026-01-05"],
                lab_received: None,
                self_processed: None,
            },
            // 2. `shot` with no shots → date_finished falls back to date_loaded.
            Scenario {
                id: 2,
                roll_id: "R-shot-noshots",
                status: "shot",
                legacy: LegacyStatus::Shot,
                date_loaded: Some("2026-02-01"),
                date_finished: None,
                date_scanned: None,
                date_post_processed: None,
                date_archived: None,
                shot_dates: &[],
                lab_received: None,
                self_processed: None,
            },
            // 3. `scanned` + a LAB dev → dev_completion (COALESCE lab branch)
            //    flows into date_scanned; date_finished from date_loaded.
            Scenario {
                id: 3,
                roll_id: "R-scanned-lab",
                status: "scanned",
                legacy: LegacyStatus::Scanned,
                date_loaded: Some("2026-03-01"),
                date_finished: None,
                date_scanned: None,
                date_post_processed: None,
                date_archived: None,
                shot_dates: &[],
                lab_received: Some("2026-03-10"),
                self_processed: None,
            },
            // 4. `scanned` + a SELF dev (no lab) → COALESCE self branch supplies
            //    dev_completion → date_scanned.
            Scenario {
                id: 4,
                roll_id: "R-scanned-self",
                status: "scanned",
                legacy: LegacyStatus::Scanned,
                date_loaded: Some("2026-04-01"),
                date_finished: None,
                date_scanned: None,
                date_post_processed: None,
                date_archived: None,
                shot_dates: &[],
                lab_received: None,
                self_processed: Some("2026-04-10"),
            },
            // 5. `archived`, no scanned/pp → date_archived chains off the
            //    date_scanned this same run derives from dev completion.
            Scenario {
                id: 5,
                roll_id: "R-archived-chain",
                status: "archived",
                legacy: LegacyStatus::Archived,
                date_loaded: Some("2026-05-01"),
                date_finished: None,
                date_scanned: None,
                date_post_processed: None,
                date_archived: None,
                shot_dates: &[],
                lab_received: None,
                self_processed: Some("2026-05-10"),
            },
            // 6. `loaded` → nothing backfilled.
            Scenario {
                id: 6,
                roll_id: "R-loaded",
                status: "loaded",
                legacy: LegacyStatus::Loaded,
                date_loaded: Some("2026-06-01"),
                date_finished: None,
                date_scanned: None,
                date_post_processed: None,
                date_archived: None,
                shot_dates: &[],
                lab_received: None,
                self_processed: None,
            },
            // 7. `archived` with every target already populated → the
            //    `WHERE ... IS NULL` guard leaves all three untouched.
            Scenario {
                id: 7,
                roll_id: "R-already-full",
                status: "archived",
                legacy: LegacyStatus::Archived,
                date_loaded: Some("2026-07-01"),
                date_finished: Some("2026-07-05"),
                date_scanned: Some("2026-07-06"),
                date_post_processed: None,
                date_archived: Some("2026-07-07"),
                shot_dates: &[],
                lab_received: None,
                self_processed: None,
            },
        ];

        let db = migrated_through_029().await;
        for s in &scenarios {
            s.seed(&db).await;
        }

        // Run THIS migration's up() against the seeded legacy rows.
        Migration.up(&SchemaManager::new(&db)).await.unwrap();

        for s in &scenarios {
            s.assert_backfilled(&db).await;
        }

        // up() must have dropped the legacy column.
        let cols = roll_columns(&db).await.unwrap();
        assert!(
            !cols.iter().any(|c| c == "status"),
            "up() must drop the legacy status column"
        );

        // Re-running up() now hits the `status`-column-gone branch: the backfill
        // block is skipped entirely, so it is a clean no-op and the dates persist.
        Migration
            .up(&SchemaManager::new(&db))
            .await
            .expect("m030 up() must be idempotent on re-run");
        for s in &scenarios {
            s.assert_backfilled(&db).await;
        }
    }
}
