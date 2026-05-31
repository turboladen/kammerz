use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── New lens mount: QBM ─────────────────────────────────────
        db.execute_unprepared(
            "INSERT OR IGNORE INTO lens_mounts (name, created_at, updated_at) VALUES
                ('QBM', datetime('now'), datetime('now'))",
        )
        .await?;

        // ── Camera: Voigtländer VSL 1 (QBM version) ────────────────
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Voigtländer', 'VSL 1', '35mm', (SELECT id FROM lens_mounts WHERE name = 'QBM'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // ── Lenses: Nikkor 200mm f/4 AI + Color-Ultron 50mm f/1.8 ──
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Nikon', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'Nikkor 200mm f/4 AI', '200', '4', datetime('now'), datetime('now')),
                ('Voigtländer', (SELECT id FROM lens_mounts WHERE name = 'QBM'), 'Color-Ultron 50mm f/1.8', '50', '1.8', datetime('now'), datetime('now'))",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Delete lenses first (no junction rows for interchangeable lenses)
        db.execute_unprepared(
            "DELETE FROM lenses WHERE
                (brand = 'Nikon' AND name_on_lens = 'Nikkor 200mm f/4 AI') OR
                (brand = 'Voigtländer' AND name_on_lens = 'Color-Ultron 50mm f/1.8')",
        )
        .await?;

        // Delete camera
        db.execute_unprepared(
            "DELETE FROM cameras WHERE brand = 'Voigtländer' AND model = 'VSL 1'
                AND lens_mount_id = (SELECT id FROM lens_mounts WHERE name = 'QBM')",
        )
        .await?;

        // Delete mount (only if no other references)
        db.execute_unprepared(
            "DELETE FROM lens_mounts WHERE name = 'QBM'
                AND NOT EXISTS (SELECT 1 FROM cameras WHERE lens_mount_id = lens_mounts.id)
                AND NOT EXISTS (SELECT 1 FROM lenses WHERE lens_mount_id = lens_mounts.id)",
        )
        .await?;

        Ok(())
    }
}
