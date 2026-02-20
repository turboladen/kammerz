use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Create lens_mounts table
        manager
            .create_table(
                Table::create()
                    .table(LensMounts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LensMounts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LensMounts::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(LensMounts::CreatedAt)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(LensMounts::UpdatedAt)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        // 2. Seed common lens mounts
        db.execute_unprepared(
            "INSERT OR IGNORE INTO lens_mounts (name, created_at, updated_at) VALUES
                ('Canon FD', datetime('now'), datetime('now')),
                ('Canon EF', datetime('now'), datetime('now')),
                ('Canon EF-S', datetime('now'), datetime('now')),
                ('Canon RF', datetime('now'), datetime('now')),
                ('Nikon F', datetime('now'), datetime('now')),
                ('Nikon Z', datetime('now'), datetime('now')),
                ('Minolta MD/MC', datetime('now'), datetime('now')),
                ('Minolta/Sony A', datetime('now'), datetime('now')),
                ('Sony E', datetime('now'), datetime('now')),
                ('Pentax K', datetime('now'), datetime('now')),
                ('Pentax 67', datetime('now'), datetime('now')),
                ('M42 (Universal)', datetime('now'), datetime('now')),
                ('Leica M', datetime('now'), datetime('now')),
                ('Leica R', datetime('now'), datetime('now')),
                ('Leica L', datetime('now'), datetime('now')),
                ('Contax/Yashica', datetime('now'), datetime('now')),
                ('Olympus OM', datetime('now'), datetime('now')),
                ('Micro Four Thirds', datetime('now'), datetime('now')),
                ('Hasselblad V', datetime('now'), datetime('now')),
                ('Mamiya 645', datetime('now'), datetime('now')),
                ('Mamiya RB/RZ67', datetime('now'), datetime('now')),
                ('Fuji X', datetime('now'), datetime('now')),
                ('Large Format (4x5+)', datetime('now'), datetime('now'))",
        )
        .await?;

        // 3. Add lens_mount_id to cameras (default 1 for fresh DBs)
        manager
            .alter_table(
                Table::alter()
                    .table(Cameras::Table)
                    .add_column(
                        ColumnDef::new(Cameras::LensMountId)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;

        // 4. Add lens_mount_id to lenses (default 1 for fresh DBs)
        manager
            .alter_table(
                Table::alter()
                    .table(Lenses::Table)
                    .add_column(
                        ColumnDef::new(Lenses::LensMountId)
                            .integer()
                            .not_null()
                            .default(1),
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
                    .table(Lenses::Table)
                    .drop_column(Lenses::LensMountId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Cameras::Table)
                    .drop_column(Cameras::LensMountId)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(LensMounts::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum LensMounts {
    Table,
    Id,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Cameras {
    Table,
    LensMountId,
}

#[derive(Iden)]
enum Lenses {
    Table,
    LensMountId,
}
