use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Trim trailing (and leading) whitespace from brand in both tables.
        // Some seed data had "Mamiya " (trailing space) causing duplicate groupBy entries.
        db.execute_unprepared("UPDATE cameras SET brand = TRIM(brand) WHERE brand != TRIM(brand)")
            .await?;
        db.execute_unprepared("UPDATE lenses SET brand = TRIM(brand) WHERE brand != TRIM(brand)")
            .await?;

        // Re-run the brand-strip fix on lens model values that were missed by migration 017
        // because the trailing space in brand prevented the LIKE match.
        db.execute_unprepared(
            "UPDATE lenses SET model = TRIM(SUBSTR(model, LENGTH(brand) + 2)) \
             WHERE model LIKE brand || ' %'",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Data cleanup — not reversible
        Ok(())
    }
}
