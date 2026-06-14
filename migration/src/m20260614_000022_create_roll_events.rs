use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RollEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RollEvents::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RollEvents::RollId).integer().not_null())
                    .col(ColumnDef::new(RollEvents::EventType).text().not_null())
                    .col(ColumnDef::new(RollEvents::FromStatus).text().null())
                    .col(ColumnDef::new(RollEvents::ToStatus).text().null())
                    .col(ColumnDef::new(RollEvents::RefKind).text().null())
                    .col(ColumnDef::new(RollEvents::RefId).integer().null())
                    .col(ColumnDef::new(RollEvents::Summary).text().not_null())
                    .col(ColumnDef::new(RollEvents::OccurredAt).text().not_null())
                    .col(ColumnDef::new(RollEvents::CreatedAt).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_roll_events_roll")
                            .from(RollEvents::Table, RollEvents::RollId)
                            .to(Rolls::Table, Rolls::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                // Composite to serve list_for_roll's `WHERE roll_id = ? ORDER BY
                // occurred_at DESC, id DESC` from the index alone (no filesort).
                Index::create()
                    .if_not_exists()
                    .name("idx_roll_events_roll_occurred")
                    .table(RollEvents::Table)
                    .col(RollEvents::RollId)
                    .col(RollEvents::OccurredAt)
                    .col(RollEvents::Id)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RollEvents::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum RollEvents {
    Table,
    Id,
    RollId,
    EventType,
    FromStatus,
    ToStatus,
    RefKind,
    RefId,
    Summary,
    OccurredAt,
    CreatedAt,
}

#[derive(Iden)]
enum Rolls {
    Table,
    Id,
}
