use crate::seed_guard::{insert_camera, insert_lens};
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
        // Guarded on (brand, model); distinct from m013's 'VSL 1 (TM)' (kammerz-vlyu.8).
        insert_camera(db, "Voigtländer", "VSL 1", "35mm", "QBM", "SLR").await?;

        // ── Lenses: Nikkor 200mm f/4 AI + Color-Ultron 50mm f/1.8 ──
        // The Color-Ultron here is on QBM; m016 seeds a same-named lens on M42 —
        // different natural key (mount differs), so both survive the guard.
        insert_lens(
            db,
            "Nikon",
            "Nikkor 200mm f/4 AI",
            "Nikon F",
            Some("200"),
            Some("4"),
        )
        .await?;
        insert_lens(
            db,
            "Voigtländer",
            "Color-Ultron 50mm f/1.8",
            "QBM",
            Some("50"),
            Some("1.8"),
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
