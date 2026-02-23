use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── Seed 20 interchangeable lenses ──────────────────────────
        // Grouped by mount system for readability.
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                -- Pentax K (1)
                ('Asahi', (SELECT id FROM lens_mounts WHERE name = 'Pentax K'), 'SMC Pentax 55mm f/2', '55', '2', datetime('now'), datetime('now')),
                -- M42 Universal (4)
                ('Asahi', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'SMC Super Takumar 50mm f/1.4', '50', '1.4', datetime('now'), datetime('now')),
                ('Asahi', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'Super-Multi-Coated TAKUMAR 35mm f/3.5', '35', '3.5', datetime('now'), datetime('now')),
                ('Asahi', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'Super-Multi-Coated TAKUMAR 105mm f/2.8', '105', '2.8', datetime('now'), datetime('now')),
                ('Helios', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), '44-2 58mm f/2', '58', '2', datetime('now'), datetime('now')),
                -- M39 / LTM (1)
                ('Industar', (SELECT id FROM lens_mounts WHERE name = 'M39 (LTM)'), 'N-61 L/D 55mm f/2.8', '55', '2.8', datetime('now'), datetime('now')),
                -- Mamiya 645 (1)
                ('Mamiya', (SELECT id FROM lens_mounts WHERE name = 'Mamiya 645'), 'Sekor C 80mm f/2.8 N', '80', '2.8', datetime('now'), datetime('now')),
                -- Mamiya Z (1)
                ('Mamiya', (SELECT id FROM lens_mounts WHERE name = 'Mamiya Z'), 'Sekor EF 35mm f/2.8', '35', '2.8', datetime('now'), datetime('now')),
                -- Minolta MD/MC (7)
                ('Minolta', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'MC Macro Rokkor QF 50mm f/3.5', '50', '3.5', datetime('now'), datetime('now')),
                ('Minolta', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'MC Rokkor-PF 55mm f/1.7', '55', '1.7', datetime('now'), datetime('now')),
                ('Minolta', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'MC Tele Rokkor-HF 30cm f/4.5', '300', '4.5', datetime('now'), datetime('now')),
                ('Minolta', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'MD Rokkor 50mm f/1.4', '50', '1.4', datetime('now'), datetime('now')),
                ('Minolta', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'MD Rokkor 50mm f/1.7', '50', '1.7', datetime('now'), datetime('now')),
                ('Minolta', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'MD Tele Rokkor-X 135mm f/2.8', '135', '2.8', datetime('now'), datetime('now')),
                ('Minolta', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'MD W.Rokkor 28mm f/2.8', '28', '2.8', datetime('now'), datetime('now')),
                -- Nikon F (4)
                ('Nikon', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), '50mm f/1.8 Series E', '50', '1.8', datetime('now'), datetime('now')),
                ('Nikon', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'Nikkor 50mm f/1.4 AF-D', '50', '1.4', datetime('now'), datetime('now')),
                ('Nikon', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'Nikkor 55mm f/1.2', '55', '1.2', datetime('now'), datetime('now')),
                ('Nikon', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'Nikkor 70-210mm f/4.0 AF-D', '70-210', '4', datetime('now'), datetime('now')),
                -- Olympus OM (1)
                ('Olympus', (SELECT id FROM lens_mounts WHERE name = 'Olympus OM'), 'F. Zuiko Auto-S 50mm f/1.8', '50', '1.8', datetime('now'), datetime('now'))",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "DELETE FROM lenses WHERE
                (brand = 'Asahi' AND name_on_lens = 'SMC Pentax 55mm f/2') OR
                (brand = 'Asahi' AND name_on_lens = 'SMC Super Takumar 50mm f/1.4') OR
                (brand = 'Asahi' AND name_on_lens = 'Super-Multi-Coated TAKUMAR 35mm f/3.5') OR
                (brand = 'Asahi' AND name_on_lens = 'Super-Multi-Coated TAKUMAR 105mm f/2.8') OR
                (brand = 'Helios' AND name_on_lens = '44-2 58mm f/2') OR
                (brand = 'Industar' AND name_on_lens = 'N-61 L/D 55mm f/2.8') OR
                (brand = 'Mamiya' AND name_on_lens = 'Sekor C 80mm f/2.8 N') OR
                (brand = 'Mamiya' AND name_on_lens = 'Sekor EF 35mm f/2.8') OR
                (brand = 'Minolta' AND name_on_lens = 'MC Macro Rokkor QF 50mm f/3.5') OR
                (brand = 'Minolta' AND name_on_lens = 'MC Rokkor-PF 55mm f/1.7') OR
                (brand = 'Minolta' AND name_on_lens = 'MC Tele Rokkor-HF 30cm f/4.5') OR
                (brand = 'Minolta' AND name_on_lens = 'MD Rokkor 50mm f/1.4') OR
                (brand = 'Minolta' AND name_on_lens = 'MD Rokkor 50mm f/1.7') OR
                (brand = 'Minolta' AND name_on_lens = 'MD Tele Rokkor-X 135mm f/2.8') OR
                (brand = 'Minolta' AND name_on_lens = 'MD W.Rokkor 28mm f/2.8') OR
                (brand = 'Nikon' AND name_on_lens = '50mm f/1.8 Series E') OR
                (brand = 'Nikon' AND name_on_lens = 'Nikkor 50mm f/1.4 AF-D') OR
                (brand = 'Nikon' AND name_on_lens = 'Nikkor 55mm f/1.2') OR
                (brand = 'Nikon' AND name_on_lens = 'Nikkor 70-210mm f/4.0 AF-D') OR
                (brand = 'Olympus' AND name_on_lens = 'F. Zuiko Auto-S 50mm f/1.8')",
        )
        .await?;

        Ok(())
    }
}
