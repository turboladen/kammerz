use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── Fix: Mamiya 90mm K/L aperture (f/4.5 → f/3.5) ──────────
        // Note: TRIM(brand) needed because pre-existing data has trailing space
        db.execute_unprepared(
            "UPDATE lenses SET max_aperture = '3.5'
             WHERE TRIM(brand) = 'Mamiya' AND name_on_lens = 'Mamiya 90mm K/L'",
        )
        .await?;

        // ── Camera: Mamiya DSX 1000 (M42 SLR) ──────────────────────
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Mamiya', 'DSX 1000', '35mm', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // ── Lenses (23 total) ───────────────────────────────────────

        // M42 Universal (4)
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Asahi', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'Super-Multi-Coated TAKUMAR 85mm f/1.8', '85', '1.8', datetime('now'), datetime('now')),
                ('Mamiya', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'Auto Mamiya/Sekor 55mm f/1.8', '55', '1.8', datetime('now'), datetime('now')),
                ('Voigtländer', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'Color-Ultron 50mm f/1.8', '50', '1.8', datetime('now'), datetime('now')),
                ('Fuji', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'EBC Fujinon-Sw 28mm f/3.5', '28', '3.5', datetime('now'), datetime('now'))",
        )
        .await?;

        // Nikon F (1) — pre-AI era
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Nippon Kogaku', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'Nikkor-S Auto 50mm f/1.4', '50', '1.4', datetime('now'), datetime('now'))",
        )
        .await?;

        // Mamiya RB/RZ67 (2)
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Mamiya', (SELECT id FROM lens_mounts WHERE name = 'Mamiya RB/RZ67'), 'Mamiya 65mm K/L', '65', '4', datetime('now'), datetime('now')),
                ('Mamiya', (SELECT id FROM lens_mounts WHERE name = 'Mamiya RB/RZ67'), 'Mamiya 180mm K/L L-A', '180', '4.5', datetime('now'), datetime('now'))",
        )
        .await?;

        // Mamiya Z (3)
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Mamiya', (SELECT id FROM lens_mounts WHERE name = 'Mamiya Z'), 'Sekor EF 50mm f/1.7', '50', '1.7', datetime('now'), datetime('now')),
                ('Mamiya', (SELECT id FROM lens_mounts WHERE name = 'Mamiya Z'), 'Sekor EF 50mm f/1.4', '50', '1.4', datetime('now'), datetime('now')),
                ('Mamiya', (SELECT id FROM lens_mounts WHERE name = 'Mamiya Z'), 'Sekor E 200mm f/4', '200', '4', datetime('now'), datetime('now'))",
        )
        .await?;

        // Contax/Yashica (3)
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax/Yashica'), 'Planar 50mm f/1.4', '50', '1.4', datetime('now'), datetime('now')),
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax/Yashica'), 'Tessar 45mm f/2.8', '45', '2.8', datetime('now'), datetime('now')),
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax/Yashica'), 'Planar 85mm f/1.4', '85', '1.4', datetime('now'), datetime('now'))",
        )
        .await?;

        // Contax RF (2)
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax RF'), 'Sonnar 50mm f/1.5', '50', '1.5', datetime('now'), datetime('now')),
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax RF'), 'Biogon 35mm f/2.8', '35', '2.8', datetime('now'), datetime('now'))",
        )
        .await?;

        // Contax G (4)
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax G'), 'Biogon 28mm f/2.8', '28', '2.8', datetime('now'), datetime('now')),
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax G'), 'Sonnar 90mm f/2.8', '90', '2.8', datetime('now'), datetime('now')),
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax G'), 'Planar 45mm f/2', '45', '2', datetime('now'), datetime('now')),
                ('Carl Zeiss', (SELECT id FROM lens_mounts WHERE name = 'Contax G'), 'Biogon 21mm f/2.8', '21', '2.8', datetime('now'), datetime('now'))",
        )
        .await?;

        // Leica R (4)
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Leica', (SELECT id FROM lens_mounts WHERE name = 'Leica R'), 'Elmarit-R 90mm f/2.8', '90', '2.8', datetime('now'), datetime('now')),
                ('Leica', (SELECT id FROM lens_mounts WHERE name = 'Leica R'), 'Summicron-R 50mm f/2', '50', '2', datetime('now'), datetime('now')),
                ('Leica', (SELECT id FROM lens_mounts WHERE name = 'Leica R'), 'Elmarit-R 35mm f/2.8', '35', '2.8', datetime('now'), datetime('now')),
                ('Leica', (SELECT id FROM lens_mounts WHERE name = 'Leica R'), 'Macro-Elmarit-R 60mm f/2.8', '60', '2.8', datetime('now'), datetime('now'))",
        )
        .await?;

        // ── Film stock: Ferrania P30 ────────────────────────────────
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, exposure_count, stock_type, iso, created_at, updated_at) VALUES
                ('Ferrania', 'P30', '135', 36, 'bw-negative', 80, datetime('now'), datetime('now'))",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Revert film stock
        db.execute_unprepared(
            "DELETE FROM film_stocks WHERE brand = 'Ferrania' AND name = 'P30' AND format = '135'",
        )
        .await?;

        // Revert lenses
        db.execute_unprepared(
            "DELETE FROM lenses WHERE
                (brand = 'Asahi' AND name_on_lens = 'Super-Multi-Coated TAKUMAR 85mm f/1.8') OR
                (brand = 'Mamiya' AND name_on_lens = 'Auto Mamiya/Sekor 55mm f/1.8') OR
                (brand = 'Voigtländer' AND name_on_lens = 'Color-Ultron 50mm f/1.8' AND lens_mount_id = (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)')) OR
                (brand = 'Fuji' AND name_on_lens = 'EBC Fujinon-Sw 28mm f/3.5') OR
                (brand = 'Nippon Kogaku' AND name_on_lens = 'Nikkor-S Auto 50mm f/1.4') OR
                (brand = 'Mamiya' AND name_on_lens = 'Mamiya 65mm K/L') OR
                (brand = 'Mamiya' AND name_on_lens = 'Mamiya 180mm K/L L-A') OR
                (brand = 'Mamiya' AND name_on_lens = 'Sekor EF 50mm f/1.7') OR
                (brand = 'Mamiya' AND name_on_lens = 'Sekor EF 50mm f/1.4') OR
                (brand = 'Mamiya' AND name_on_lens = 'Sekor E 200mm f/4') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Planar 50mm f/1.4') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Tessar 45mm f/2.8') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Planar 85mm f/1.4') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Sonnar 50mm f/1.5') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Biogon 35mm f/2.8') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Biogon 28mm f/2.8') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Sonnar 90mm f/2.8') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Planar 45mm f/2') OR
                (brand = 'Carl Zeiss' AND name_on_lens = 'Biogon 21mm f/2.8') OR
                (brand = 'Leica' AND name_on_lens = 'Elmarit-R 90mm f/2.8') OR
                (brand = 'Leica' AND name_on_lens = 'Summicron-R 50mm f/2') OR
                (brand = 'Leica' AND name_on_lens = 'Elmarit-R 35mm f/2.8') OR
                (brand = 'Leica' AND name_on_lens = 'Macro-Elmarit-R 60mm f/2.8')",
        )
        .await?;

        // Revert camera
        db.execute_unprepared("DELETE FROM cameras WHERE brand = 'Mamiya' AND model = 'DSX 1000'")
            .await?;

        // Revert Mamiya 90mm K/L aperture fix
        db.execute_unprepared(
            "UPDATE lenses SET max_aperture = '4.5'
             WHERE brand = 'Mamiya' AND name_on_lens = 'Mamiya 90mm K/L'",
        )
        .await?;

        Ok(())
    }
}
