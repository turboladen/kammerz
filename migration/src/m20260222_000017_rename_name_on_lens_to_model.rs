use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Rename column name_on_lens → model
        db.execute_unprepared("ALTER TABLE lenses RENAME COLUMN name_on_lens TO model")
            .await?;

        // 2. Strip leading brand from model values where brand was duplicated
        //    e.g. brand="Mamiya", model="Mamiya 90mm K/L" → "90mm K/L"
        db.execute_unprepared(
            "UPDATE lenses SET model = TRIM(SUBSTR(model, LENGTH(brand) + 2)) \
             WHERE model LIKE brand || ' %'",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Restore brand prefix on model values (inverse of step 2 in up())
        db.execute_unprepared(
            "UPDATE lenses SET model = brand || ' ' || model \
             WHERE model IS NOT NULL AND model != '' \
             AND NOT (model LIKE brand || ' %')",
        )
        .await?;

        db.execute_unprepared("ALTER TABLE lenses RENAME COLUMN model TO name_on_lens")
            .await?;
        Ok(())
    }
}
