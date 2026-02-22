use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add nullable default_lens_id column to cameras table.
        // Note: SQLite ALTER TABLE cannot add FK constraints,
        // so referential integrity is enforced at the app layer
        // via the SeaORM BelongsTo relation on camera::Entity.
        manager
            .alter_table(
                Table::alter()
                    .table(Cameras::Table)
                    .add_column(ColumnDef::new(Cameras::DefaultLensId).integer().null())
                    .to_owned(),
            )
            .await?;

        // Backfill: for cameras with exactly one linked lens, set it as default.
        // This covers existing fixed-lens cameras that were manually linked.
        let db = manager.get_connection();
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (
                SELECT cl.lens_id FROM camera_lenses cl
                WHERE cl.camera_id = cameras.id
                GROUP BY cl.camera_id
                HAVING COUNT(*) = 1
            )",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Cameras::Table)
                    .drop_column(Cameras::DefaultLensId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Cameras {
    Table,
    DefaultLensId,
}
