use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Per-shot time-of-day, captured from historical notes (e.g. "7:27pm") and
        // the Quick Entry form. Stored as nullable TEXT in canonical 24-hour HH:MM
        // (enforced by validate_time), complementing the existing `date` column.
        manager
            .alter_table(
                Table::alter()
                    .table(Shots::Table)
                    .add_column(ColumnDef::new(Shots::Time).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Shots::Table)
                    .drop_column(Shots::Time)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Shots {
    Table,
    Time,
}
