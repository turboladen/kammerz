use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Repair migration: SQLx sets PRAGMA foreign_keys=ON by default on SQLite
/// connections. Migration 019 (schema_hardening) rebuilt the cameras and lenses
/// tables using DROP TABLE + RENAME, which triggered SQLite's implicit DELETE
/// cascading through ON DELETE CASCADE (camera_lenses) and ON DELETE SET NULL
/// (cameras.default_lens_id), destroying all fixed-lens associations.
///
/// This migration re-populates camera_lenses junction entries and
/// default_lens_id for all fixed-lens cameras seeded in migration 013.

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Re-populate camera_lenses junction entries.
        // Uses INSERT OR IGNORE so this is safe to re-run.
        // Each fixed-lens camera is matched to its lens by brand + model name.
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Meopta' AND model = 'Flexaret III'),
                 (SELECT id FROM lenses WHERE brand = 'Meopta' AND model = 'Belar 80mm f/3.5')),
                ((SELECT id FROM cameras WHERE brand = 'Voigtländer' AND model = 'Brilliant'),
                 (SELECT id FROM lenses WHERE brand = 'Voigtländer' AND model = 'Voigtar 75mm f/7.7')),
                ((SELECT id FROM cameras WHERE brand = 'Yashica' AND model = 'Electro 35 GSN'),
                 (SELECT id FROM lenses WHERE brand = 'Yashica' AND model = 'Yashinon-DX 45mm f/1.7')),
                ((SELECT id FROM cameras WHERE brand = 'Kodak' AND model = 'Chevron'),
                 (SELECT id FROM lenses WHERE brand = 'Kodak' AND model = 'Ektar 78mm f/3.5')),
                ((SELECT id FROM cameras WHERE brand = 'Kodak' AND model = 'No. 2 Brownie, Model D'),
                 (SELECT id FROM lenses WHERE brand = 'Kodak' AND model = 'Meniscus')),
                ((SELECT id FROM cameras WHERE brand = 'Rollei' AND model = 'XF 35'),
                 (SELECT id FROM lenses WHERE brand = 'Rollei' AND model = 'Sonnar 40mm f/2.3')),
                ((SELECT id FROM cameras WHERE brand = 'Nikon' AND model = '35Ti'),
                 (SELECT id FROM lenses WHERE brand = 'Nikon' AND model = 'Nikkor 35mm f/2.8')),
                ((SELECT id FROM cameras WHERE brand = 'Olympus' AND model = 'Stylus Epic Zoom 80'),
                 (SELECT id FROM lenses WHERE brand = 'Olympus' AND model = '38-80mm f/4.5')),
                ((SELECT id FROM cameras WHERE brand = 'Olympus' AND model = '35RD'),
                 (SELECT id FROM lenses WHERE brand = 'Olympus' AND model = 'F.Zuiko 40mm f/1.7')),
                ((SELECT id FROM cameras WHERE brand = 'Minolta' AND model = 'Hi-Matic F'),
                 (SELECT id FROM lenses WHERE brand = 'Minolta' AND model = 'Rokkor 38mm f/2.7')),
                ((SELECT id FROM cameras WHERE brand = 'Canon' AND model = 'Canonet G-III QL'),
                 (SELECT id FROM lenses WHERE brand = 'Canon' AND model = 'Lens 40mm f/1.7')),
                ((SELECT id FROM cameras WHERE brand = 'Zeiss Ikon' AND model = 'S310'),
                 (SELECT MIN(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND model = 'Tessar 50mm f/2.8')),
                ((SELECT id FROM cameras WHERE brand = 'Zeiss Ikon' AND model = 'Contessamat SBE'),
                 (SELECT MAX(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND model = 'Tessar 50mm f/2.8')),
                ((SELECT id FROM cameras WHERE brand = 'Diana' AND model = 'F'),
                 (SELECT id FROM lenses WHERE brand = 'Diana' AND model = '75mm f/11'))",
        )
        .await?;

        // Re-populate default_lens_id for each fixed-lens camera.
        // Only updates if currently NULL to avoid overwriting any manual changes.
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (
                SELECT cl.lens_id FROM camera_lenses cl WHERE cl.camera_id = cameras.id LIMIT 1
             )
             WHERE default_lens_id IS NULL
               AND lens_mount_id = (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens')",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Reverse: clear the repaired data
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = NULL
             WHERE lens_mount_id = (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens')",
        )
        .await?;

        db.execute_unprepared(
            "DELETE FROM camera_lenses WHERE camera_id IN (
                SELECT id FROM cameras
                WHERE lens_mount_id = (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens')
            )",
        )
        .await?;

        Ok(())
    }
}
