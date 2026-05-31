use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Polaroid 600 (for 600-series cameras: OneStep, Sun 660, etc.)
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Polaroid', '600 Color', 'instant', 8, 'color-negative', 640),
                ('Polaroid', '600 B&W', 'instant', 8, 'bw-negative', 640)"
        ).await?;

        // Polaroid i-Type (for i-Type cameras: Now, Now+, Go Generation 2)
        // Same chemistry as 600 but no battery in cartridge
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Polaroid', 'i-Type Color', 'instant', 8, 'color-negative', 640),
                ('Polaroid', 'i-Type B&W', 'instant', 8, 'bw-negative', 640)"
        ).await?;

        // Polaroid SX-70 (for SX-70 folding SLR cameras)
        // Slower emulsion (~ISO 160) than 600/i-Type
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Polaroid', 'SX-70 Color', 'instant', 8, 'color-negative', 160),
                ('Polaroid', 'SX-70 B&W', 'instant', 8, 'bw-negative', 160)"
        ).await?;

        // Polaroid Go (for Polaroid Go mini cameras)
        // Smallest Polaroid format; sold as double packs but 8 per cartridge
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Polaroid', 'Go Color', 'instant', 8, 'color-negative', 640)"
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "DELETE FROM film_stocks WHERE brand = 'Polaroid' AND format = 'instant'"
        ).await?;

        Ok(())
    }
}
