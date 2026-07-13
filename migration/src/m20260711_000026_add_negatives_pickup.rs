use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Per-lab negative retention policy (days). NULL → app treats as the
        // default 30 (encoded as COALESCE in the roll-list query); no backfill.
        manager
            .alter_table(
                Table::alter()
                    .table(Labs::Table)
                    .add_column(ColumnDef::new(Labs::NegativeRetentionDays).integer().null())
                    .to_owned(),
            )
            .await?;

        // Physical-possession date (distinct from date_received, which is the
        // lab's "order ready" notification).
        manager
            .alter_table(
                Table::alter()
                    .table(DevelopmentLabs::Table)
                    .add_column(
                        ColumnDef::new(DevelopmentLabs::DateNegativesPickedUp)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // "Not collecting" opt-out. NOT NULL DEFAULT 0 so existing rows read false.
        manager
            .alter_table(
                Table::alter()
                    .table(DevelopmentLabs::Table)
                    .add_column(
                        ColumnDef::new(DevelopmentLabs::NegativesNotCollecting)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DevelopmentLabs::Table)
                    .drop_column(DevelopmentLabs::NegativesNotCollecting)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DevelopmentLabs::Table)
                    .drop_column(DevelopmentLabs::DateNegativesPickedUp)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Labs::Table)
                    .drop_column(Labs::NegativeRetentionDays)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Labs {
    Table,
    NegativeRetentionDays,
}

#[derive(Iden)]
enum DevelopmentLabs {
    Table,
    DateNegativesPickedUp,
    NegativesNotCollecting,
}
