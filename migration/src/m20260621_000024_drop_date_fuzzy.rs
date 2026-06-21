use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
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
