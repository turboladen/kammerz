use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── 1. Missing indexes on foreign key columns ───────────────────
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_cameras_lens_mount ON cameras(lens_mount_id)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_lenses_lens_mount ON lenses(lens_mount_id)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_dev_lab_lab ON development_lab(lab_id)",
        )
        .await?;

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
        db.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_dev_lab_roll_unique ON development_lab(roll_id)",
        )
        .await?;
        db.execute_unprepared("CREATE UNIQUE INDEX IF NOT EXISTS idx_dev_self_roll_unique ON development_self(roll_id)").await?;

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
        db.execute_unprepared(
            "UPDATE camera_maintenance SET maintenance_type = 'other'
             WHERE maintenance_type NOT IN ('CLA','repair','cleaning','modification','other')",
        )
        .await?;

        // ── 3. Rebuild lens_mounts to fix timestamp defaults ────────────
        //    Old defaults were empty string ""; should be datetime('now').
        db.execute_unprepared(
            "CREATE TABLE lens_mounts_new (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
        .await?;
        db.execute_unprepared(
            "INSERT INTO lens_mounts_new (id, name, created_at, updated_at)
             SELECT id, name,
                    CASE WHEN created_at = '' THEN datetime('now') ELSE created_at END,
                    CASE WHEN updated_at = '' THEN datetime('now') ELSE updated_at END
             FROM lens_mounts",
        )
        .await?;
        db.execute_unprepared("DROP TABLE lens_mounts").await?;
        db.execute_unprepared("ALTER TABLE lens_mounts_new RENAME TO lens_mounts")
            .await?;

        // ── 4. Rebuild cameras to add FK constraints + fix defaults ─────
        //    lens_mount_id and default_lens_id were added via ALTER TABLE
        //    and have no FK constraints. Also fixes hardcoded DEFAULT 1.
        db.execute_unprepared(
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
        )
        .await?;
        db.execute_unprepared(
            "INSERT INTO cameras_new (id, brand, model, prefix, format, lens_mount_id,
                default_lens_id, camera_type, serial_number, date_purchased,
                purchased_from, date_sold, notes, created_at, updated_at)
             SELECT id, brand, model, prefix, format, lens_mount_id,
                default_lens_id, camera_type, serial_number, date_purchased,
                purchased_from, date_sold, notes, created_at, updated_at
             FROM cameras",
        )
        .await?;
        db.execute_unprepared("DROP TABLE cameras").await?;
        db.execute_unprepared("ALTER TABLE cameras_new RENAME TO cameras")
            .await?;
        // Recreate index lost during rebuild
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_cameras_lens_mount ON cameras(lens_mount_id)",
        )
        .await?;

        // ── 5. Rebuild rolls to add FK constraint on lens_id ────────────
        //    lens_id was added via ALTER TABLE in migration 004 without FK.
        db.execute_unprepared(
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
        )
        .await?;
        db.execute_unprepared(
            "INSERT INTO rolls_new (id, roll_id, camera_id, film_stock_id, lens_id,
                status, frame_count, date_loaded, date_finished, date_fuzzy,
                push_pull, notes, created_at, updated_at)
             SELECT id, roll_id, camera_id, film_stock_id, lens_id,
                status, frame_count, date_loaded, date_finished, date_fuzzy,
                push_pull, notes, created_at, updated_at
             FROM rolls",
        )
        .await?;
        db.execute_unprepared("DROP TABLE rolls").await?;
        db.execute_unprepared("ALTER TABLE rolls_new RENAME TO rolls")
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
        db.execute_unprepared(
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
        )
        .await?;
        db.execute_unprepared(
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
        db.execute_unprepared("DROP TABLE lenses").await?;
        db.execute_unprepared("ALTER TABLE lenses_new RENAME TO lenses")
            .await?;
        // Recreate index
        db.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS idx_lenses_lens_mount ON lenses(lens_mount_id)",
        )
        .await?;

        // ── 7. Rename singular tables to plural for consistency ──────────
        db.execute_unprepared("ALTER TABLE development_lab RENAME TO development_labs")
            .await?;
        db.execute_unprepared("ALTER TABLE development_self RENAME TO development_selves")
            .await?;
        db.execute_unprepared("ALTER TABLE camera_maintenance RENAME TO camera_maintenances")
            .await?;

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
