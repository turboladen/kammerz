use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_rolls_lens")
                    .table(Rolls::Table)
                    .col(Rolls::LensId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_rolls_lens").to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Rolls {
    Table,
    LensId,
}
