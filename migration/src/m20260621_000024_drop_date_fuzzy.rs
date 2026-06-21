use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Deliberately raw `ALTER TABLE ... DROP COLUMN` (SQLite 3.35+), NOT the
    // `manager.alter_table(...).drop_column(...)` form used by the simple ALTERs
    // in migrations 021/023. `date_fuzzy` lives on the `rolls` and `shots` PARENT
    // tables; a native DROP COLUMN edits the schema in place, whereas a
    // builder-emitted column drop can fall back to a full table rebuild
    // (CREATE new → copy → DROP old → RENAME) — the exact pattern that lost data
    // pre-pragma-fix and needed the migration-020 repair (see backend-patterns.md).
    // Caveat: `execute_unprepared` auto-commits each statement, so a crash between
    // the two DROPs leaves one applied and the migration unrecorded; a re-run then
    // errors "no such column". SQLite has no `DROP COLUMN IF EXISTS` guard.
    // Acceptable for a single-user catalog with backups; recover by re-adding the
    // already-dropped column by hand and re-running.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("ALTER TABLE rolls DROP COLUMN date_fuzzy")
            .await?;
        conn.execute_unprepared("ALTER TABLE shots DROP COLUMN date_fuzzy")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("ALTER TABLE rolls ADD COLUMN date_fuzzy TEXT")
            .await?;
        conn.execute_unprepared("ALTER TABLE shots ADD COLUMN date_fuzzy TEXT")
            .await?;
        Ok(())
    }
}
