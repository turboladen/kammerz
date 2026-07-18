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

/// Legacy `RollStatus` values in progression order. A status's index is its rank;
/// the backfill fills a date once a roll reached at least the ranked milestone.
pub const BACKFILL_ORDER: &[&str] = &[
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
];

fn rank(status: &str) -> Option<usize> {
    BACKFILL_ORDER.iter().position(|s| *s == status)
}

fn rank_of(status: &str) -> usize {
    rank(status).expect("BACKFILL_ORDER contains the milestone")
}

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
#[allow(clippy::too_many_arguments)]
pub fn backfilled_dates(
    status: &str,
    date_loaded: Option<&str>,
    date_finished: Option<&str>,
    max_shot_date: Option<&str>,
    dev_completion: Option<&str>,
    date_scanned: Option<&str>,
    date_post_processed: Option<&str>,
    date_archived: Option<&str>,
) -> BackfilledDates {
    let Some(r) = rank(status) else {
        return BackfilledDates::default();
    };
    let mut out = BackfilledDates::default();

    // status >= shot: date_finished := max(shot dates) ?? date_loaded
    if r >= rank_of("shot") && date_finished.is_none() {
        out.date_finished = max_shot_date.or(date_loaded).map(str::to_string);
    }

    // status >= scanned: date_scanned := date_post_processed ?? dev completion
    if r >= rank_of("scanned") && date_scanned.is_none() {
        out.date_scanned = date_post_processed.or(dev_completion).map(str::to_string);
    }

    // status >= archived: date_archived := date_post_processed ?? date_scanned
    // (existing or the value we just chose above) ?? dev completion
    if r >= rank_of("archived") && date_archived.is_none() {
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

                let filled = backfilled_dates(
                    &status,
                    date_loaded.as_deref(),
                    date_finished.as_deref(),
                    max_shot_date.as_deref(),
                    dev_completion.as_deref(),
                    date_scanned.as_deref(),
                    date_post_processed.as_deref(),
                    date_archived.as_deref(),
                );

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

    #[test]
    fn loaded_and_shooting_get_no_dates() {
        for status in ["loaded", "shooting"] {
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
            assert_eq!(out, BackfilledDates::default(), "{status}");
        }
    }

    #[test]
    fn shot_backfills_date_finished_from_max_shot_date() {
        let out = backfilled_dates(
            "shot",
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
            "shot",
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
        for status in ["at-lab", "lab-done", "developing", "developed"] {
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
            assert_eq!(out.date_finished.as_deref(), Some("2026-01-05"), "{status}");
            assert_eq!(out.date_scanned, None, "{status}");
            assert_eq!(out.date_archived, None, "{status}");
        }
    }

    #[test]
    fn scanned_backfills_from_post_processed_then_dev_completion() {
        let from_pp = backfilled_dates(
            "scanned",
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
            "scanned",
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
            "archived",
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
            "archived",
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
            "archived",
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
        let out = backfilled_dates("archived", None, None, None, None, None, None, None);
        assert_eq!(out, BackfilledDates::default());
    }

    #[test]
    fn unknown_status_is_a_noop() {
        let out = backfilled_dates(
            "bogus",
            Some("2026-01-01"),
            None,
            Some("2026-01-05"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(out, BackfilledDates::default());
    }
}
