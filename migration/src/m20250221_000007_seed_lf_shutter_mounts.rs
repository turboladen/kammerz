use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "INSERT OR IGNORE INTO lens_mounts (name, created_at, updated_at) VALUES
                ('Copal #0', datetime('now'), datetime('now')),
                ('Copal #1', datetime('now'), datetime('now')),
                ('Copal #3', datetime('now'), datetime('now')),
                ('Compur #0', datetime('now'), datetime('now')),
                ('Compur #1', datetime('now'), datetime('now')),
                ('Compur #3', datetime('now'), datetime('now')),
                ('Barrel Mount (no shutter)', datetime('now'), datetime('now'))",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "DELETE FROM lens_mounts WHERE name IN (
                'Copal #0', 'Copal #1', 'Copal #3',
                'Compur #0', 'Compur #1', 'Compur #3',
                'Barrel Mount (no shutter)'
            )",
        )
        .await?;

        Ok(())
    }
}
