use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add nullable lens_id column to rolls table
        // Note: SQLite cannot add FK constraints via ALTER TABLE,
        // so referential integrity is enforced at the application layer
        // via the SeaORM BelongsTo relation on roll::Entity.
        manager
            .alter_table(
                Table::alter()
                    .table(Rolls::Table)
                    .add_column(ColumnDef::new(Rolls::LensId).integer().null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Rolls::Table)
                    .drop_column(Rolls::LensId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Rolls {
    Table,
    LensId,
}
