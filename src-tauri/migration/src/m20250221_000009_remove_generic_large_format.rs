use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Reassign any cameras/lenses using "Large Format" to "Barrel Mount (no shutter)"
        db.execute_unprepared(
            "UPDATE cameras SET lens_mount_id = (
                SELECT id FROM lens_mounts WHERE name = 'Barrel Mount (no shutter)'
             ) WHERE lens_mount_id = (
                SELECT id FROM lens_mounts WHERE name = 'Large Format'
             )",
        )
        .await?;

        db.execute_unprepared(
            "UPDATE lenses SET lens_mount_id = (
                SELECT id FROM lens_mounts WHERE name = 'Barrel Mount (no shutter)'
             ) WHERE lens_mount_id = (
                SELECT id FROM lens_mounts WHERE name = 'Large Format'
             )",
        )
        .await?;

        db.execute_unprepared("DELETE FROM lens_mounts WHERE name = 'Large Format'")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Re-insert the generic entry
        db.execute_unprepared(
            "INSERT OR IGNORE INTO lens_mounts (name, created_at, updated_at)
             VALUES ('Large Format', datetime('now'), datetime('now'))",
        )
        .await?;

        Ok(())
    }
}
