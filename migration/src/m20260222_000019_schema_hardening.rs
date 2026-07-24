//! Schema hardening: add missing FK-column indexes, UNIQUE dev-per-roll indexes,
//! normalize enum columns, rebuild four tables (to fix timestamp defaults and add
//! FK constraints that `ALTER TABLE` couldn't), and rename the singular dev/
//! maintenance tables to plural.
//!
//! Re-run safety (kammerz-vlyu.9): SQLite migrations run WITHOUT an enclosing
//! transaction and `execute_unprepared` auto-commits each statement, so a crash
//! mid-`up()` leaves this migration unrecorded and it re-runs from statement 1 on
//! the next boot. The four table rebuilds (CREATE `_new` → copy → DROP old →
//! RENAME) and the section-7 renames were NOT re-runnable — a second pass hit
//! `CREATE TABLE cameras_new` (already exists) or `RENAME development_lab` (no
//! longer exists) and returned a `DbErr`, wedging startup with no self-heal.
//!
//! The rebuilds now go through [`rebuild_table`], a small state machine that reads
//! each table's live `sqlite_master.sql` and resumes from any crash point:
//!
//! - live table already carries its post-rebuild marker → skip (drop any stale
//!   `_new` a prior crash left behind),
//! - live table still in the old shape → clear any partial `_new`, full rebuild,
//! - live table gone but `_new` present (crashed between DROP and RENAME) →
//!   finish the rename.
//!
//! On a clean first run every table is in the old shape, so the "full rebuild"
//! branch runs for all four — byte-identical to the original migration's output.
//! A section-0 pre-pass first finishes any rebuild that crashed after `DROP old`
//! but before `RENAME` (only `{name}_new` present), because sections 1/2b below
//! reference cameras/lenses/rolls by name before their own rebuilds run — without
//! the pre-pass a re-run from that window would fail on a missing table.
//!
//! The per-table "upgraded" markers are substrings that appear ONLY post-rebuild:
//! `datetime('now')` in the created_at default (old default was `''`) for
//! `lens_mounts`, and the FK clause the rebuild adds for the others
//! (`REFERENCES lens_mounts` for `cameras`/`lenses`, `REFERENCES lenses` for
//! `rolls`; the pre-rebuild `lens_id`/`lens_mount_id` columns were `ALTER`-added
//! without a FK). The section-7 renames use [`rename_table_if_needed`].

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Fetch a table's stored `CREATE TABLE` SQL from `sqlite_master`, or `None` if
/// no such table exists. The state detection below keys off this.
async fn table_sql(db: &impl ConnectionTrait, name: &str) -> Result<Option<String>, DbErr> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = $1",
            [name.to_owned().into()],
        ))
        .await?;
    match row {
        Some(r) => Ok(Some(r.try_get::<String>("", "sql")?)),
        None => Ok(None),
    }
}

/// Resumable table rebuild (CREATE `{name}_new` → copy → DROP `{name}` → RENAME).
/// `upgraded_marker` is a substring present ONLY in the post-rebuild table SQL, so
/// a re-run can tell "already done" from "not started". Any partial `{name}_new`
/// from a prior crash is dropped before a fresh rebuild so the copy can't collide
/// with half-written rows. Idempotent and crash-resumable from every point.
async fn rebuild_table(
    db: &impl ConnectionTrait,
    name: &str,
    upgraded_marker: &str,
    create_new_sql: &str,
    copy_sql: &str,
) -> Result<(), DbErr> {
    let new_name = format!("{name}_new");
    let live = table_sql(db, name).await?;
    let new_exists = table_sql(db, &new_name).await?.is_some();

    match live {
        // Already rebuilt — skip. Drop any stale `_new` a crash left behind.
        Some(ref sql) if sql.contains(upgraded_marker) => {
            if new_exists {
                db.execute_unprepared(&format!("DROP TABLE IF EXISTS {new_name}"))
                    .await?;
            }
        }
        // Old shape still present (never rebuilt, or crashed before the DROP):
        // clear any partial `_new`, then do the full rebuild.
        Some(_) => {
            db.execute_unprepared(&format!("DROP TABLE IF EXISTS {new_name}"))
                .await?;
            db.execute_unprepared(create_new_sql).await?;
            db.execute_unprepared(copy_sql).await?;
            db.execute_unprepared(&format!("DROP TABLE IF EXISTS {name}"))
                .await?;
            db.execute_unprepared(&format!("ALTER TABLE {new_name} RENAME TO {name}"))
                .await?;
        }
        // Original gone but `_new` present: crashed between DROP and RENAME — finish.
        None if new_exists => {
            db.execute_unprepared(&format!("ALTER TABLE {new_name} RENAME TO {name}"))
                .await?;
        }
        // Neither `{name}` nor `{name}_new` exists — the section-0 pre-pass in
        // `up()` guarantees the live table is present by the time this runs, so
        // this is unreachable. Fail loud rather than silently continuing startup
        // with a missing table.
        None => {
            return Err(DbErr::Custom(format!(
                "rebuild_table: neither `{name}` nor `{new_name}` exists"
            )));
        }
    }
    Ok(())
}

/// Rename `from` → `to` only if `from` still exists and `to` doesn't, so a re-run
/// after the rename already happened is a no-op instead of a "no such table" error.
async fn rename_table_if_needed(
    db: &impl ConnectionTrait,
    from: &str,
    to: &str,
) -> Result<(), DbErr> {
    let from_exists = table_sql(db, from).await?.is_some();
    let to_exists = table_sql(db, to).await?.is_some();
    if from_exists && !to_exists {
        db.execute_unprepared(&format!("ALTER TABLE {from} RENAME TO {to}"))
            .await?;
    }
    Ok(())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── 0. Finish any rebuild that crashed between DROP old and RENAME ──
        //    The rebuilds in sections 3-6 leave a window where the original table
        //    is dropped but `{name}_new` is not yet renamed back. Sections 1 and 2b
        //    below reference cameras/lenses/rolls by name BEFORE those rebuilds run,
        //    so without this pass a re-run from that window would fail on a missing
        //    table (`CREATE INDEX ... ON cameras`, `UPDATE rolls ...`) and never
        //    reach the rebuild that would finish the rename — wedging startup, the
        //    exact failure this migration is meant to make recoverable. Completing
        //    the pending rename first closes that hole for every rebuilt table.
        //    No-op on a clean first run (no `*_new` tables exist).
        for t in ["lens_mounts", "cameras", "rolls", "lenses"] {
            rename_table_if_needed(db, &format!("{t}_new"), t).await?;
        }

        // ── 1. Missing indexes on foreign key columns ───────────────────
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_cameras_lens_mount ON cameras(lens_mount_id)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_lenses_lens_mount ON lenses(lens_mount_id)",
        )
        .await?;
        // Guarded on the singular table's existence: section 7 renames
        // development_lab → development_labs, and on a post-rename re-run this
        // index already exists in final form. `CREATE INDEX IF NOT EXISTS` does
        // NOT tolerate a missing table reference, so skip it once renamed.
        if table_sql(db, "development_lab").await?.is_some() {
            db.execute_unprepared(
                "CREATE INDEX IF NOT EXISTS idx_dev_lab_lab ON development_lab(lab_id)",
            )
            .await?;
        }

        // Reverse-lookup indexes on junction tables (composite PK only covers leading column)
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_camera_lenses_lens ON camera_lenses(lens_id)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_shot_lenses_lens ON shot_lenses(lens_id)",
        )
        .await?;

        // ── 2. UNIQUE constraints on development tables (one dev per roll) ──
        //    These UNIQUE indexes supersede the non-unique idx_dev_*_roll
        //    created in the initial migration — drop those first.
        db.execute_unprepared("DROP INDEX IF EXISTS idx_dev_lab_roll")
            .await?;
        db.execute_unprepared("DROP INDEX IF EXISTS idx_dev_self_roll")
            .await?;
        // Guarded on the singular table names for the same reason as idx_dev_lab_lab
        // above (section 7 renames them; a post-rename re-run already has these).
        if table_sql(db, "development_lab").await?.is_some() {
            db.execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_dev_lab_roll_unique ON development_lab(roll_id)",
            )
            .await?;
        }
        if table_sql(db, "development_self").await?.is_some() {
            db.execute_unprepared("CREATE UNIQUE INDEX IF NOT EXISTS idx_dev_self_roll_unique ON development_self(roll_id)").await?;
        }

        // ── 2b. Normalize enum columns to valid values ────────────────
        //    DeriveActiveEnum will reject any value outside the enum set.
        //    Clean up any stale data before queries start using enums.
        db.execute_unprepared(
            "UPDATE rolls SET status = 'loaded'
             WHERE status NOT IN ('loaded','shooting','shot','at-lab','developing','developed','scanned','archived')"
        ).await?;
        db.execute_unprepared(
            "UPDATE rolls SET push_pull = NULL
             WHERE push_pull IS NOT NULL AND push_pull NOT IN ('-2','-1','+1','+2','+3')",
        )
        .await?;
        db.execute_unprepared(
            "UPDATE cameras SET format = '35mm'
             WHERE format NOT IN ('35mm','medium format','6x4.5','6x6','6x7','6x8','6x9','large format','4x5','5x7','8x10','instant')"
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET camera_type = NULL
             WHERE camera_type IS NOT NULL AND camera_type NOT IN ('SLR','rangefinder','TLR','point-and-shoot','box','view','instant')"
        ).await?;
        db.execute_unprepared(
            "UPDATE film_stocks SET format = '135'
             WHERE format NOT IN ('135','120','4x5','5x7','8x10','instant')",
        )
        .await?;
        db.execute_unprepared(
            "UPDATE film_stocks SET stock_type = 'color-negative'
             WHERE stock_type NOT IN ('color-negative','bw-negative','color-slide','bw-slide')",
        )
        .await?;
        // Guarded on the singular table name: section 7 renames camera_maintenance
        // → camera_maintenances. On a post-rename re-run the rows are already
        // normalized under the plural name, so skip this UPDATE.
        if table_sql(db, "camera_maintenance").await?.is_some() {
            db.execute_unprepared(
                "UPDATE camera_maintenance SET maintenance_type = 'other'
                 WHERE maintenance_type NOT IN ('CLA','repair','cleaning','modification','other')",
            )
            .await?;
        }

        // ── 3. Rebuild lens_mounts to fix timestamp defaults ────────────
        //    Old defaults were empty string ""; should be datetime('now').
        rebuild_table(
            db,
            "lens_mounts",
            "datetime('now')",
            "CREATE TABLE lens_mounts_new (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            "INSERT INTO lens_mounts_new (id, name, created_at, updated_at)
             SELECT id, name,
                    CASE WHEN created_at = '' THEN datetime('now') ELSE created_at END,
                    CASE WHEN updated_at = '' THEN datetime('now') ELSE updated_at END
             FROM lens_mounts",
        )
        .await?;

        // ── 4. Rebuild cameras to add FK constraints + fix defaults ─────
        //    lens_mount_id and default_lens_id were added via ALTER TABLE
        //    and have no FK constraints. Also fixes hardcoded DEFAULT 1.
        rebuild_table(
            db,
            "cameras",
            "REFERENCES lens_mounts",
            "CREATE TABLE cameras_new (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                brand TEXT NOT NULL,
                model TEXT NOT NULL,
                prefix TEXT,
                format TEXT NOT NULL,
                lens_mount_id INTEGER NOT NULL REFERENCES lens_mounts(id),
                default_lens_id INTEGER REFERENCES lenses(id) ON DELETE SET NULL,
                camera_type TEXT,
                serial_number TEXT,
                date_purchased TEXT,
                purchased_from TEXT,
                date_sold TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            "INSERT INTO cameras_new (id, brand, model, prefix, format, lens_mount_id,
                default_lens_id, camera_type, serial_number, date_purchased,
                purchased_from, date_sold, notes, created_at, updated_at)
             SELECT id, brand, model, prefix, format, lens_mount_id,
                default_lens_id, camera_type, serial_number, date_purchased,
                purchased_from, date_sold, notes, created_at, updated_at
             FROM cameras",
        )
        .await?;
        // Recreate index lost during rebuild
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_cameras_lens_mount ON cameras(lens_mount_id)",
        )
        .await?;

        // ── 5. Rebuild rolls to add FK constraint on lens_id ────────────
        //    lens_id was added via ALTER TABLE in migration 004 without FK.
        rebuild_table(
            db,
            "rolls",
            "REFERENCES lenses",
            "CREATE TABLE rolls_new (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                roll_id TEXT NOT NULL UNIQUE,
                camera_id INTEGER REFERENCES cameras(id) ON DELETE SET NULL,
                film_stock_id INTEGER REFERENCES film_stocks(id) ON DELETE SET NULL,
                lens_id INTEGER REFERENCES lenses(id) ON DELETE SET NULL,
                status TEXT NOT NULL DEFAULT 'loaded',
                frame_count INTEGER,
                date_loaded TEXT,
                date_finished TEXT,
                date_fuzzy TEXT,
                push_pull TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            "INSERT INTO rolls_new (id, roll_id, camera_id, film_stock_id, lens_id,
                status, frame_count, date_loaded, date_finished, date_fuzzy,
                push_pull, notes, created_at, updated_at)
             SELECT id, roll_id, camera_id, film_stock_id, lens_id,
                status, frame_count, date_loaded, date_finished, date_fuzzy,
                push_pull, notes, created_at, updated_at
             FROM rolls",
        )
        .await?;
        // Recreate indexes lost during rebuild
        db.execute_unprepared("CREATE INDEX IF NOT EXISTS idx_rolls_camera ON rolls(camera_id)")
            .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_rolls_film_stock ON rolls(film_stock_id)",
        )
        .await?;
        db.execute_unprepared("CREATE INDEX IF NOT EXISTS idx_rolls_status ON rolls(status)")
            .await?;
        db.execute_unprepared("CREATE INDEX IF NOT EXISTS idx_rolls_lens ON rolls(lens_id)")
            .await?;

        // ── 6. Rebuild lenses to add FK constraint on lens_mount_id ─────
        //    lens_mount_id was added via ALTER TABLE in migration 006.
        rebuild_table(
            db,
            "lenses",
            "REFERENCES lens_mounts",
            "CREATE TABLE lenses_new (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                brand TEXT NOT NULL,
                lens_mount_id INTEGER NOT NULL REFERENCES lens_mounts(id),
                lens_system TEXT,
                model TEXT,
                focal_length TEXT,
                max_aperture TEXT,
                min_aperture TEXT,
                filter_thread_front_mm INTEGER,
                filter_thread_rear_mm INTEGER,
                serial_number TEXT,
                date_purchased TEXT,
                purchased_from TEXT,
                date_sold TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            "INSERT INTO lenses_new (id, brand, lens_mount_id, lens_system, model,
                focal_length, max_aperture, min_aperture, filter_thread_front_mm,
                filter_thread_rear_mm, serial_number, date_purchased,
                purchased_from, date_sold, notes, created_at, updated_at)
             SELECT id, brand, lens_mount_id, lens_system, model,
                focal_length, max_aperture, min_aperture, filter_thread_front_mm,
                filter_thread_rear_mm, serial_number, date_purchased,
                purchased_from, date_sold, notes, created_at, updated_at
             FROM lenses",
        )
        .await?;
        // Recreate index
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_lenses_lens_mount ON lenses(lens_mount_id)",
        )
        .await?;

        // ── 7. Rename singular tables to plural for consistency ──────────
        rename_table_if_needed(db, "development_lab", "development_labs").await?;
        rename_table_if_needed(db, "development_self", "development_selves").await?;
        rename_table_if_needed(db, "camera_maintenance", "camera_maintenances").await?;

        // Recreate indexes that reference old table names
        // (SQLite RENAME TABLE automatically updates indexes, but unique indexes
        //  created with old names still work — just naming consistency)

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Destructive migration — no rollback support
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Migrator;
    use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection};

    /// A fresh in-memory DB migrated through m019 (and no further), with FK
    /// enforcement OFF exactly as `db.rs` sets it before migrating. Stopping at
    /// m019 keeps the schema in the state a crash would actually re-enter from —
    /// notably `rolls.status` still exists (m030 drops it later), so the
    /// section-2b normalization on re-run is realistic. A single connection keeps
    /// the in-memory DB alive across statements.
    async fn migrated_through_019() -> DatabaseConnection {
        let mut opt = ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1).min_connections(1);
        let db = Database::connect(opt).await.unwrap();
        db.execute_unprepared("PRAGMA foreign_keys=OFF")
            .await
            .unwrap();
        // Some(19): apply the first 19 migrations (through m019), none after.
        Migrator::up(&db, Some(19)).await.unwrap();
        db
    }

    #[tokio::test]
    async fn m019_up_reruns_cleanly() {
        let db = migrated_through_019().await;

        // Sanity: all four rebuilt tables carry their post-rebuild marker.
        assert!(
            table_sql(&db, "lens_mounts")
                .await
                .unwrap()
                .unwrap()
                .contains("datetime('now')")
        );
        assert!(
            table_sql(&db, "cameras")
                .await
                .unwrap()
                .unwrap()
                .contains("REFERENCES lens_mounts")
        );
        assert!(
            table_sql(&db, "rolls")
                .await
                .unwrap()
                .unwrap()
                .contains("REFERENCES lenses")
        );
        assert!(
            table_sql(&db, "lenses")
                .await
                .unwrap()
                .unwrap()
                .contains("REFERENCES lens_mounts")
        );

        // Re-running up() is the real post-crash path. Before the fix this
        // returned a DbErr (section-7 renames failed the second time); it must
        // now be a clean no-op.
        let manager = SchemaManager::new(&db);
        Migration
            .up(&manager)
            .await
            .expect("m019 up() must be idempotent on re-run");

        // Exactly one of each table, still upgraded, and no `_new` leftovers.
        for t in ["lens_mounts", "cameras", "rolls", "lenses"] {
            assert!(
                table_sql(&db, t).await.unwrap().is_some(),
                "{t} missing after re-run"
            );
            assert!(
                table_sql(&db, &format!("{t}_new")).await.unwrap().is_none(),
                "{t}_new leftover after re-run"
            );
        }
        for t in [
            "development_labs",
            "development_selves",
            "camera_maintenances",
        ] {
            assert!(
                table_sql(&db, t).await.unwrap().is_some(),
                "{t} missing after re-run"
            );
        }
    }

    #[tokio::test]
    async fn m019_resumes_when_crashed_before_rename() {
        // Simulate a crash in the "DROP old → before RENAME" window for the three
        // tables that sections 1/2b reference by name BEFORE their own rebuild
        // (cameras via idx_cameras_lens_mount, lenses via idx_lenses_lens_mount,
        // rolls via the section-2b UPDATEs). Renaming each live table to its `_new`
        // form reproduces that state: `{name}` gone, `{name}_new` present. Without
        // the section-0 pre-pass, up() would fail on a missing table before ever
        // reaching the rebuild that finishes the rename.
        let db = migrated_through_019().await;
        for t in ["cameras", "rolls", "lenses"] {
            db.execute_unprepared(&format!("ALTER TABLE {t} RENAME TO {t}_new"))
                .await
                .unwrap();
            assert!(table_sql(&db, t).await.unwrap().is_none());
        }

        let manager = SchemaManager::new(&db);
        Migration
            .up(&manager)
            .await
            .expect("m019 up() must resume from a pre-RENAME crash");

        for t in ["cameras", "rolls", "lenses"] {
            assert!(
                table_sql(&db, t).await.unwrap().is_some(),
                "{t} not restored after resume"
            );
            assert!(
                table_sql(&db, &format!("{t}_new")).await.unwrap().is_none(),
                "{t}_new leftover after resume"
            );
        }
    }

    #[tokio::test]
    async fn rebuild_table_resumes_from_every_state() {
        let mut opt = ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1).min_connections(1);
        let db = Database::connect(opt).await.unwrap();

        let create_new =
            "CREATE TABLE t_new (id INTEGER PRIMARY KEY, v TEXT, tag TEXT DEFAULT 'up')";
        let copy = "INSERT INTO t_new (id, v) SELECT id, v FROM t";
        let marker = "tag TEXT DEFAULT 'up'";

        // Old-shape table with a row to prove data survives the rebuild.
        db.execute_unprepared("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
            .await
            .unwrap();
        db.execute_unprepared("INSERT INTO t (id, v) VALUES (1, 'a')")
            .await
            .unwrap();

        // (1) Old shape present → full rebuild, data preserved.
        rebuild_table(&db, "t", marker, create_new, copy)
            .await
            .unwrap();
        assert!(table_sql(&db, "t").await.unwrap().unwrap().contains(marker));
        assert!(table_sql(&db, "t_new").await.unwrap().is_none());
        let v: String = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT v FROM t WHERE id = 1".to_owned(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "v")
            .unwrap();
        assert_eq!(v, "a", "row must survive the rebuild");

        // (2) Already upgraded → skip, and a stale `t_new` gets cleared.
        db.execute_unprepared("CREATE TABLE t_new (x INTEGER)")
            .await
            .unwrap();
        rebuild_table(&db, "t", marker, create_new, copy)
            .await
            .unwrap();
        assert!(table_sql(&db, "t").await.unwrap().unwrap().contains(marker));
        assert!(
            table_sql(&db, "t_new").await.unwrap().is_none(),
            "stale t_new must be dropped"
        );

        // (3) Crashed between DROP old and RENAME: only `t_new` exists → finish.
        db.execute_unprepared("ALTER TABLE t RENAME TO t_new")
            .await
            .unwrap();
        assert!(table_sql(&db, "t").await.unwrap().is_none());
        rebuild_table(&db, "t", marker, create_new, copy)
            .await
            .unwrap();
        assert!(table_sql(&db, "t").await.unwrap().unwrap().contains(marker));
        assert!(table_sql(&db, "t_new").await.unwrap().is_none());
    }
}
