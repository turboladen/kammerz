use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Capture the remaining roll-lifecycle milestones that previously had no
        // date column: scanned, post-processed, archived. Stored as nullable TEXT
        // (ISO YYYY-MM-DD), matching date_loaded / date_finished. Auto-stamped on
        // the matching status transition by the frontend, editable in the roll Edit
        // form. SQLite permits only one ADD COLUMN per ALTER TABLE, so each column
        // is added in its own statement.
        for column in [Rolls::DateScanned, Rolls::DatePostProcessed, Rolls::DateArchived] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Rolls::Table)
                        .add_column(ColumnDef::new(column).text().null())
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in [Rolls::DateScanned, Rolls::DatePostProcessed, Rolls::DateArchived] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Rolls::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(Iden)]
enum Rolls {
    Table,
    DateScanned,
    DatePostProcessed,
    DateArchived,
}
