use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "UPDATE lens_mounts SET name = 'Large Format', updated_at = datetime('now')
             WHERE name = 'Large Format (4x5+)'",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "UPDATE lens_mounts SET name = 'Large Format (4x5+)', updated_at = datetime('now')
             WHERE name = 'Large Format'",
        )
        .await?;

        Ok(())
    }
}
