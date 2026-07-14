//! Canonical developer/chemistry reference table + seed (kammerz-9fx).
//!
//! `development_selves` keeps its free-text chemistry columns (no FK); this table
//! is an autocomplete reference that the self-dev save path self-learns into
//! (`ChemicalService::upsert_from_self_dev`). The seed names are BYTE-EXACT the
//! canonical `to` targets of the m..029 normalization (plus CineStill CS41), so a
//! normalized record always matches a suggested option — `tests/chemicals.rs`
//! asserts that alignment.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Chemicals::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Chemicals::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Chemicals::Name).text().not_null())
                    .col(ColumnDef::new(Chemicals::Type).text().not_null())
                    .col(ColumnDef::new(Chemicals::DefaultDilution).text().null())
                    .col(ColumnDef::new(Chemicals::CreatedAt).text().not_null())
                    .col(ColumnDef::new(Chemicals::UpdatedAt).text().not_null())
                    .to_owned(),
            )
            .await?;

        // UNIQUE(name, type) — the auto-upsert and seed both rely on it for
        // idempotent `INSERT OR IGNORE`.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("idx_chemicals_name_type")
                    .table(Chemicals::Table)
                    .col(Chemicals::Name)
                    .col(Chemicals::Type)
                    .to_owned(),
            )
            .await?;

        // Seed canonical products. `INSERT OR IGNORE` is idempotent via the unique
        // index. `default_dilution` only where a sensible default exists.
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT OR IGNORE INTO chemicals (name, type, default_dilution, created_at, updated_at) VALUES
                    ('Kodak D-76', 'developer', 'stock', datetime('now'), datetime('now')),
                    ('Kodak XTOL', 'developer', '1+1', datetime('now'), datetime('now')),
                    ('Adox Rodinal', 'developer', '1+25', datetime('now'), datetime('now')),
                    ('FPP Monobath', 'developer', NULL, datetime('now'), datetime('now')),
                    ('CineStill CS41', 'developer', NULL, datetime('now'), datetime('now')),
                    ('Kodak Fixer', 'fixer', '1+4', datetime('now'), datetime('now')),
                    ('Kodak Stop Bath', 'stop_bath', NULL, datetime('now'), datetime('now')),
                    ('Kodak Photo-Flo 200', 'wetting_agent', NULL, datetime('now'), datetime('now')),
                    ('Photographers Formulary Hyper Flow', 'wetting_agent', NULL, datetime('now'), datetime('now')),
                    ('Kodak Hypo Clearing Agent', 'clearing_agent', NULL, datetime('now'), datetime('now'))",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Chemicals::Table).if_exists().to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Chemicals {
    Table,
    Id,
    Name,
    Type,
    DefaultDilution,
    CreatedAt,
    UpdatedAt,
}
