use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Normalize shots.aperture to a bare f-number (e.g. "2.8"), stripping the
        // legacy "f/" prefix left by the original bulk import. The app is designed
        // for bare storage: the entry paths (QuickAddBar, import) store the bare
        // number and every display site prepends "f/". The mixed convention made
        // f/-prefixed rows render as "f/f/2.8". Idempotent (the LIKE guard means it
        // no-ops once applied), so it's safe on already-normalized databases.
        db.execute_unprepared(
            "UPDATE shots SET aperture = SUBSTR(aperture, 3) WHERE aperture LIKE 'f/%'",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Data normalization — not reversible.
        Ok(())
    }
}
