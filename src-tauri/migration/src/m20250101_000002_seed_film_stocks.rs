use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Color Negative - 35mm
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Kodak', 'Portra 160', '135', 36, 'color-negative', 160),
                ('Kodak', 'Portra 400', '135', 36, 'color-negative', 400),
                ('Kodak', 'Portra 800', '135', 36, 'color-negative', 800),
                ('Kodak', 'Ektar 100', '135', 36, 'color-negative', 100),
                ('Kodak', 'Gold 200', '135', 36, 'color-negative', 200),
                ('Kodak', 'Ultramax 400', '135', 36, 'color-negative', 400),
                ('Kodak', 'ColorPlus 200', '135', 36, 'color-negative', 200),
                ('Fujifilm', 'Superia X-TRA 400', '135', 36, 'color-negative', 400),
                ('Fujifilm', 'C200', '135', 36, 'color-negative', 200),
                ('Fujifilm', 'Pro 400H', '135', 36, 'color-negative', 400)"
        ).await?;

        // Color Negative - 120
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Kodak', 'Portra 160', '120', NULL, 'color-negative', 160),
                ('Kodak', 'Portra 400', '120', NULL, 'color-negative', 400),
                ('Kodak', 'Portra 800', '120', NULL, 'color-negative', 800),
                ('Kodak', 'Ektar 100', '120', NULL, 'color-negative', 100),
                ('Fujifilm', 'Pro 400H', '120', NULL, 'color-negative', 400)"
        ).await?;

        // B&W Negative - 35mm
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Kodak', 'Tri-X 400', '135', 36, 'bw-negative', 400),
                ('Kodak', 'T-Max 100', '135', 36, 'bw-negative', 100),
                ('Kodak', 'T-Max 400', '135', 36, 'bw-negative', 400),
                ('Kodak', 'T-Max P3200', '135', 36, 'bw-negative', 3200),
                ('Ilford', 'HP5 Plus', '135', 36, 'bw-negative', 400),
                ('Ilford', 'FP4 Plus', '135', 36, 'bw-negative', 125),
                ('Ilford', 'Delta 100', '135', 36, 'bw-negative', 100),
                ('Ilford', 'Delta 400', '135', 36, 'bw-negative', 400),
                ('Ilford', 'Delta 3200', '135', 36, 'bw-negative', 3200),
                ('Ilford', 'Pan F Plus', '135', 36, 'bw-negative', 50),
                ('Ilford', 'XP2 Super', '135', 36, 'bw-negative', 400),
                ('Foma', 'Fomapan 100', '135', 36, 'bw-negative', 100),
                ('Foma', 'Fomapan 200', '135', 36, 'bw-negative', 200),
                ('Foma', 'Fomapan 400', '135', 36, 'bw-negative', 400)"
        ).await?;

        // B&W Negative - 120
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Kodak', 'Tri-X 400', '120', NULL, 'bw-negative', 400),
                ('Kodak', 'T-Max 100', '120', NULL, 'bw-negative', 100),
                ('Kodak', 'T-Max 400', '120', NULL, 'bw-negative', 400),
                ('Ilford', 'HP5 Plus', '120', NULL, 'bw-negative', 400),
                ('Ilford', 'FP4 Plus', '120', NULL, 'bw-negative', 125),
                ('Ilford', 'Delta 100', '120', NULL, 'bw-negative', 100),
                ('Ilford', 'Delta 400', '120', NULL, 'bw-negative', 400),
                ('Ilford', 'Delta 3200', '120', NULL, 'bw-negative', 3200),
                ('Foma', 'Fomapan 100', '120', NULL, 'bw-negative', 100),
                ('Foma', 'Fomapan 200', '120', NULL, 'bw-negative', 200),
                ('Foma', 'Fomapan 400', '120', NULL, 'bw-negative', 400)"
        ).await?;

        // Color Slide - 35mm
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Kodak', 'Ektachrome E100', '135', 36, 'color-slide', 100),
                ('Fujifilm', 'Velvia 50', '135', 36, 'color-slide', 50),
                ('Fujifilm', 'Velvia 100', '135', 36, 'color-slide', 100),
                ('Fujifilm', 'Provia 100F', '135', 36, 'color-slide', 100)"
        ).await?;

        // Color Slide - 120
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Kodak', 'Ektachrome E100', '120', NULL, 'color-slide', 100),
                ('Fujifilm', 'Velvia 50', '120', NULL, 'color-slide', 50),
                ('Fujifilm', 'Velvia 100', '120', NULL, 'color-slide', 100),
                ('Fujifilm', 'Provia 100F', '120', NULL, 'color-slide', 100)"
        ).await?;

        // Large Format Sheets - 4x5
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso) VALUES
                ('Kodak', 'Portra 160', '4x5', 1, 'color-negative', 160),
                ('Kodak', 'Portra 400', '4x5', 1, 'color-negative', 400),
                ('Kodak', 'Ektar 100', '4x5', 1, 'color-negative', 100),
                ('Kodak', 'Ektachrome E100', '4x5', 1, 'color-slide', 100),
                ('Kodak', 'Tri-X 400', '4x5', 1, 'bw-negative', 400),
                ('Kodak', 'T-Max 100', '4x5', 1, 'bw-negative', 100),
                ('Kodak', 'T-Max 400', '4x5', 1, 'bw-negative', 400),
                ('Ilford', 'HP5 Plus', '4x5', 1, 'bw-negative', 400),
                ('Ilford', 'FP4 Plus', '4x5', 1, 'bw-negative', 125),
                ('Ilford', 'Delta 100', '4x5', 1, 'bw-negative', 100),
                ('Foma', 'Fomapan 100', '4x5', 1, 'bw-negative', 100)"
        ).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Seed data removal not needed — the schema drop handles it
        Ok(())
    }
}
