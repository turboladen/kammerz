use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ── cameras ─────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Cameras::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Cameras::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Cameras::Brand).text().not_null())
                    .col(ColumnDef::new(Cameras::Model).text().not_null())
                    .col(ColumnDef::new(Cameras::Prefix).text())
                    .col(ColumnDef::new(Cameras::Format).text().not_null())
                    .col(ColumnDef::new(Cameras::CameraType).text())
                    .col(ColumnDef::new(Cameras::SerialNumber).text())
                    .col(ColumnDef::new(Cameras::DatePurchased).text())
                    .col(ColumnDef::new(Cameras::PurchasedFrom).text())
                    .col(ColumnDef::new(Cameras::DateSold).text())
                    .col(ColumnDef::new(Cameras::Notes).text())
                    .col(
                        ColumnDef::new(Cameras::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(Cameras::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .to_owned(),
            )
            .await?;

        // ── camera_maintenance ──────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(CameraMaintenance::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CameraMaintenance::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CameraMaintenance::CameraId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CameraMaintenance::MaintenanceType)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(CameraMaintenance::DoneBy).text())
                    .col(ColumnDef::new(CameraMaintenance::DateDone).text())
                    .col(ColumnDef::new(CameraMaintenance::Cost).double())
                    .col(ColumnDef::new(CameraMaintenance::Notes).text())
                    .col(
                        ColumnDef::new(CameraMaintenance::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(CameraMaintenance::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(CameraMaintenance::Table, CameraMaintenance::CameraId)
                            .to(Cameras::Table, Cameras::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_camera_maintenance_camera")
                    .table(CameraMaintenance::Table)
                    .col(CameraMaintenance::CameraId)
                    .to_owned(),
            )
            .await?;

        // ── lenses ──────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Lenses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Lenses::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Lenses::Brand).text().not_null())
                    .col(ColumnDef::new(Lenses::LensSystem).text())
                    .col(ColumnDef::new(Lenses::NameOnLens).text())
                    .col(ColumnDef::new(Lenses::FocalLength).text())
                    .col(ColumnDef::new(Lenses::MaxAperture).text())
                    .col(ColumnDef::new(Lenses::MinAperture).text())
                    .col(ColumnDef::new(Lenses::FilterThreadFrontMm).integer())
                    .col(ColumnDef::new(Lenses::FilterThreadRearMm).integer())
                    .col(ColumnDef::new(Lenses::SerialNumber).text())
                    .col(ColumnDef::new(Lenses::DatePurchased).text())
                    .col(ColumnDef::new(Lenses::PurchasedFrom).text())
                    .col(ColumnDef::new(Lenses::DateSold).text())
                    .col(ColumnDef::new(Lenses::Notes).text())
                    .col(
                        ColumnDef::new(Lenses::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(Lenses::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .to_owned(),
            )
            .await?;

        // ── camera_lenses (junction) ────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(CameraLenses::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(CameraLenses::CameraId).integer().not_null())
                    .col(ColumnDef::new(CameraLenses::LensId).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(CameraLenses::CameraId)
                            .col(CameraLenses::LensId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(CameraLenses::Table, CameraLenses::CameraId)
                            .to(Cameras::Table, Cameras::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(CameraLenses::Table, CameraLenses::LensId)
                            .to(Lenses::Table, Lenses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // ── film_stocks ─────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(FilmStocks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FilmStocks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FilmStocks::Brand).text().not_null())
                    .col(ColumnDef::new(FilmStocks::Name).text().not_null())
                    .col(ColumnDef::new(FilmStocks::Format).text().not_null())
                    .col(ColumnDef::new(FilmStocks::ExposureCount).integer())
                    .col(ColumnDef::new(FilmStocks::StockType).text().not_null())
                    .col(ColumnDef::new(FilmStocks::Iso).integer())
                    .col(ColumnDef::new(FilmStocks::Notes).text())
                    .col(
                        ColumnDef::new(FilmStocks::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(FilmStocks::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .to_owned(),
            )
            .await?;

        // UNIQUE(brand, name, format) on film_stocks
        manager
            .create_index(
                Index::create()
                    .name("idx_film_stocks_unique")
                    .table(FilmStocks::Table)
                    .col(FilmStocks::Brand)
                    .col(FilmStocks::Name)
                    .col(FilmStocks::Format)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // ── labs ────────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Labs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Labs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Labs::Name).text().not_null())
                    .col(ColumnDef::new(Labs::Location).text())
                    .col(ColumnDef::new(Labs::Website).text())
                    .col(ColumnDef::new(Labs::Notes).text())
                    .col(
                        ColumnDef::new(Labs::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(Labs::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .to_owned(),
            )
            .await?;

        // ── rolls ───────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Rolls::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Rolls::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Rolls::RollId).text().not_null().unique_key())
                    .col(ColumnDef::new(Rolls::CameraId).integer())
                    .col(ColumnDef::new(Rolls::FilmStockId).integer())
                    .col(
                        ColumnDef::new(Rolls::Status)
                            .text()
                            .not_null()
                            .default("loaded"),
                    )
                    .col(ColumnDef::new(Rolls::FrameCount).integer())
                    .col(ColumnDef::new(Rolls::DateLoaded).text())
                    .col(ColumnDef::new(Rolls::DateFinished).text())
                    .col(ColumnDef::new(Rolls::DateFuzzy).text())
                    .col(ColumnDef::new(Rolls::PushPull).text())
                    .col(ColumnDef::new(Rolls::Notes).text())
                    .col(
                        ColumnDef::new(Rolls::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(Rolls::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Rolls::Table, Rolls::CameraId)
                            .to(Cameras::Table, Cameras::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Rolls::Table, Rolls::FilmStockId)
                            .to(FilmStocks::Table, FilmStocks::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_rolls_camera")
                    .table(Rolls::Table)
                    .col(Rolls::CameraId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_rolls_film_stock")
                    .table(Rolls::Table)
                    .col(Rolls::FilmStockId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_rolls_status")
                    .table(Rolls::Table)
                    .col(Rolls::Status)
                    .to_owned(),
            )
            .await?;

        // ── shots ───────────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(Shots::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Shots::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Shots::RollId).integer().not_null())
                    .col(ColumnDef::new(Shots::FrameNumber).text().not_null())
                    .col(ColumnDef::new(Shots::Aperture).text())
                    .col(ColumnDef::new(Shots::ShutterSpeed).text())
                    .col(ColumnDef::new(Shots::Date).text())
                    .col(ColumnDef::new(Shots::DateFuzzy).text())
                    .col(ColumnDef::new(Shots::Location).text())
                    .col(ColumnDef::new(Shots::GpsLat).double())
                    .col(ColumnDef::new(Shots::GpsLon).double())
                    .col(ColumnDef::new(Shots::Notes).text())
                    .col(
                        ColumnDef::new(Shots::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(Shots::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Shots::Table, Shots::RollId)
                            .to(Rolls::Table, Rolls::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_shots_roll")
                    .table(Shots::Table)
                    .col(Shots::RollId)
                    .to_owned(),
            )
            .await?;

        // UNIQUE(roll_id, frame_number) on shots
        manager
            .create_index(
                Index::create()
                    .name("idx_shots_roll_frame")
                    .table(Shots::Table)
                    .col(Shots::RollId)
                    .col(Shots::FrameNumber)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // ── shot_lenses (junction) ──────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(ShotLenses::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ShotLenses::ShotId).integer().not_null())
                    .col(ColumnDef::new(ShotLenses::LensId).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(ShotLenses::ShotId)
                            .col(ShotLenses::LensId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ShotLenses::Table, ShotLenses::ShotId)
                            .to(Shots::Table, Shots::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ShotLenses::Table, ShotLenses::LensId)
                            .to(Lenses::Table, Lenses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // ── development_lab ─────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(DevelopmentLab::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DevelopmentLab::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DevelopmentLab::RollId).integer().not_null())
                    .col(ColumnDef::new(DevelopmentLab::LabId).integer())
                    .col(ColumnDef::new(DevelopmentLab::DateDroppedOff).text())
                    .col(ColumnDef::new(DevelopmentLab::DateReceived).text())
                    .col(ColumnDef::new(DevelopmentLab::Cost).double())
                    .col(ColumnDef::new(DevelopmentLab::Notes).text())
                    .col(
                        ColumnDef::new(DevelopmentLab::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(DevelopmentLab::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(DevelopmentLab::Table, DevelopmentLab::RollId)
                            .to(Rolls::Table, Rolls::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(DevelopmentLab::Table, DevelopmentLab::LabId)
                            .to(Labs::Table, Labs::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dev_lab_roll")
                    .table(DevelopmentLab::Table)
                    .col(DevelopmentLab::RollId)
                    .to_owned(),
            )
            .await?;

        // ── development_self ────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(DevelopmentSelf_::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DevelopmentSelf_::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DevelopmentSelf_::RollId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(DevelopmentSelf_::DateProcessed).text())
                    .col(ColumnDef::new(DevelopmentSelf_::Developer).text())
                    .col(ColumnDef::new(DevelopmentSelf_::DeveloperDilution).text())
                    .col(ColumnDef::new(DevelopmentSelf_::Fixer).text())
                    .col(ColumnDef::new(DevelopmentSelf_::FixerDilution).text())
                    .col(ColumnDef::new(DevelopmentSelf_::StopBath).text())
                    .col(ColumnDef::new(DevelopmentSelf_::WettingAgent).text())
                    .col(ColumnDef::new(DevelopmentSelf_::ClearingAgent).text())
                    .col(ColumnDef::new(DevelopmentSelf_::Temperature).text())
                    .col(ColumnDef::new(DevelopmentSelf_::AgitationNotes).text())
                    .col(ColumnDef::new(DevelopmentSelf_::Notes).text())
                    .col(
                        ColumnDef::new(DevelopmentSelf_::CreatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .col(
                        ColumnDef::new(DevelopmentSelf_::UpdatedAt)
                            .text()
                            .not_null()
                            .default(Expr::cust("(datetime('now'))")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(DevelopmentSelf_::Table, DevelopmentSelf_::RollId)
                            .to(Rolls::Table, Rolls::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dev_self_roll")
                    .table(DevelopmentSelf_::Table)
                    .col(DevelopmentSelf_::RollId)
                    .to_owned(),
            )
            .await?;

        // ── dev_stages ──────────────────────────────────────────
        manager
            .create_table(
                Table::create()
                    .table(DevStages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DevStages::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(DevStages::DevelopmentSelfId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(DevStages::StageName).text().not_null())
                    .col(ColumnDef::new(DevStages::DurationSeconds).integer())
                    .col(ColumnDef::new(DevStages::Notes).text())
                    .col(
                        ColumnDef::new(DevStages::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(DevStages::Table, DevStages::DevelopmentSelfId)
                            .to(DevelopmentSelf_::Table, DevelopmentSelf_::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_dev_stages_dev")
                    .table(DevStages::Table)
                    .col(DevStages::DevelopmentSelfId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DevStages::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DevelopmentSelf_::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DevelopmentLab::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ShotLenses::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Shots::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Rolls::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Labs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(FilmStocks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CameraLenses::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Lenses::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CameraMaintenance::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Cameras::Table).to_owned())
            .await?;
        Ok(())
    }
}

// ── Iden enums ──────────────────────────────────────────────────

#[derive(Iden)]
enum Cameras {
    Table,
    Id,
    Brand,
    Model,
    Prefix,
    Format,
    CameraType,
    SerialNumber,
    DatePurchased,
    PurchasedFrom,
    DateSold,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum CameraMaintenance {
    Table,
    Id,
    CameraId,
    MaintenanceType,
    DoneBy,
    DateDone,
    Cost,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Lenses {
    Table,
    Id,
    Brand,
    LensSystem,
    NameOnLens,
    FocalLength,
    MaxAperture,
    MinAperture,
    FilterThreadFrontMm,
    FilterThreadRearMm,
    SerialNumber,
    DatePurchased,
    PurchasedFrom,
    DateSold,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum CameraLenses {
    Table,
    CameraId,
    LensId,
}

#[derive(Iden)]
enum FilmStocks {
    Table,
    Id,
    Brand,
    Name,
    Format,
    ExposureCount,
    StockType,
    Iso,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Labs {
    Table,
    Id,
    Name,
    Location,
    Website,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Rolls {
    Table,
    Id,
    RollId,
    CameraId,
    FilmStockId,
    Status,
    FrameCount,
    DateLoaded,
    DateFinished,
    DateFuzzy,
    PushPull,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Shots {
    Table,
    Id,
    RollId,
    FrameNumber,
    Aperture,
    ShutterSpeed,
    Date,
    DateFuzzy,
    Location,
    GpsLat,
    GpsLon,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ShotLenses {
    Table,
    ShotId,
    LensId,
}

#[derive(Iden)]
enum DevelopmentLab {
    Table,
    Id,
    RollId,
    LabId,
    DateDroppedOff,
    DateReceived,
    Cost,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
#[allow(clippy::enum_variant_names)]
enum DevelopmentSelf_ {
    Table,
    Id,
    RollId,
    DateProcessed,
    Developer,
    DeveloperDilution,
    Fixer,
    FixerDilution,
    StopBath,
    WettingAgent,
    ClearingAgent,
    Temperature,
    AgitationNotes,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum DevStages {
    Table,
    Id,
    DevelopmentSelfId,
    StageName,
    DurationSeconds,
    Notes,
    SortOrder,
}

#[cfg(test)]
mod tests {
    use crate::Migrator;
    use sea_orm_migration::prelude::*;
    use sea_orm_migration::sea_orm::{
        ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement,
    };

    async fn fresh_db() -> DatabaseConnection {
        let mut opt = ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1).min_connections(1);
        let db = Database::connect(opt).await.unwrap();
        db.execute_unprepared("PRAGMA foreign_keys=OFF")
            .await
            .unwrap();
        db
    }

    async fn table_sql(db: &DatabaseConnection, name: &str) -> String {
        db.query_one(Statement::from_string(
            db.get_database_backend(),
            format!("SELECT sql AS v FROM sqlite_master WHERE type = 'table' AND name = '{name}'"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "v")
        .unwrap()
    }

    /// kammerz-2v2h: a `&str` passed to `.default()` is emitted by sea-query as a
    /// QUOTED STRING LITERAL, so `datetime('now')` would be stored as literal text.
    /// The fix uses `Expr::cust`, which emits a real SQL expression default. The
    /// tables m019 never rebuilds (only renames) are where the bug would persist —
    /// assert their created m001 DDL carries the expression form, not the literal.
    /// At Some(1) the dev/maintenance tables still have their pre-m019 singular names.
    #[tokio::test]
    async fn initial_schema_timestamp_defaults_are_expressions() {
        let db = fresh_db().await;
        Migrator::up(&db, Some(1)).await.unwrap();

        for t in [
            "camera_maintenance",
            "film_stocks",
            "labs",
            "shots",
            "development_lab",
            "development_self",
        ] {
            let sql = table_sql(&db, t).await;
            assert!(
                sql.contains("(datetime('now'))"),
                "{t}: timestamp default must be the datetime() expression, got: {sql}"
            );
            assert!(
                !sql.contains("'datetime(''now'')'"),
                "{t}: timestamp default must NOT be a quoted string literal, got: {sql}"
            );
        }
    }

    /// End-to-end proof of the fix: on the full schema, inserting into a non-rebuilt
    /// table WITHOUT the timestamps must evaluate `datetime('now')` to a real
    /// timestamp — not store the literal text (which is what the bug did).
    #[tokio::test]
    async fn omitted_timestamp_defaults_to_real_datetime() {
        let db = fresh_db().await;
        Migrator::up(&db, None).await.unwrap();

        db.execute_unprepared("INSERT INTO labs (name) VALUES ('default-probe')")
            .await
            .unwrap();
        let created = db
            .query_one(Statement::from_string(
                db.get_database_backend(),
                "SELECT created_at AS v FROM labs WHERE name = 'default-probe'".to_owned(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<String>("", "v")
            .unwrap();

        assert_ne!(
            created, "datetime('now')",
            "default stored the literal text instead of a timestamp"
        );
        // datetime('now') renders as 'YYYY-MM-DD HH:MM:SS'.
        assert_eq!(
            created.len(),
            19,
            "expected a datetime string, got: {created}"
        );
        assert!(
            created.as_bytes()[0].is_ascii_digit()
                && created.contains('-')
                && created.contains(':'),
            "expected a datetime string, got: {created}"
        );
    }
}
