use crate::seed_guard::{insert_camera, insert_lens};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── Fix: Mamiya 90mm K/L aperture (f/4.5 → f/3.5) ──────────
        // Live-data fix (no-op on a fresh DB — the row is imported later). Idempotent.
        // Note: TRIM(brand) needed because pre-existing data has trailing space
        db.execute_unprepared(
            "UPDATE lenses SET max_aperture = '3.5'
             WHERE TRIM(brand) = 'Mamiya' AND name_on_lens = 'Mamiya 90mm K/L'",
        )
        .await?;

        // ── Camera: Mamiya DSX 1000 (M42 SLR) ──────────────────────
        // Guarded on (brand, model) so a crash re-run doesn't duplicate (kammerz-vlyu.8).
        insert_camera(db, "Mamiya", "DSX 1000", "35mm", "M42 (Universal)", "SLR").await?;

        // ── Lenses (23 total) ───────────────────────────────────────
        // Each guarded on the pre-m017 natural key (brand, name_on_lens, mount).

        // M42 Universal (4)
        insert_lens(
            db,
            "Asahi",
            "Super-Multi-Coated TAKUMAR 85mm f/1.8",
            "M42 (Universal)",
            Some("85"),
            Some("1.8"),
        )
        .await?;
        insert_lens(
            db,
            "Mamiya",
            "Auto Mamiya/Sekor 55mm f/1.8",
            "M42 (Universal)",
            Some("55"),
            Some("1.8"),
        )
        .await?;
        insert_lens(
            db,
            "Voigtländer",
            "Color-Ultron 50mm f/1.8",
            "M42 (Universal)",
            Some("50"),
            Some("1.8"),
        )
        .await?;
        insert_lens(
            db,
            "Fuji",
            "EBC Fujinon-Sw 28mm f/3.5",
            "M42 (Universal)",
            Some("28"),
            Some("3.5"),
        )
        .await?;

        // Nikon F (1) — pre-AI era
        insert_lens(
            db,
            "Nippon Kogaku",
            "Nikkor-S Auto 50mm f/1.4",
            "Nikon F",
            Some("50"),
            Some("1.4"),
        )
        .await?;

        // Mamiya RB/RZ67 (2)
        insert_lens(
            db,
            "Mamiya",
            "Mamiya 65mm K/L",
            "Mamiya RB/RZ67",
            Some("65"),
            Some("4"),
        )
        .await?;
        insert_lens(
            db,
            "Mamiya",
            "Mamiya 180mm K/L L-A",
            "Mamiya RB/RZ67",
            Some("180"),
            Some("4.5"),
        )
        .await?;

        // Mamiya Z (3)
        insert_lens(
            db,
            "Mamiya",
            "Sekor EF 50mm f/1.7",
            "Mamiya Z",
            Some("50"),
            Some("1.7"),
        )
        .await?;
        insert_lens(
            db,
            "Mamiya",
            "Sekor EF 50mm f/1.4",
            "Mamiya Z",
            Some("50"),
            Some("1.4"),
        )
        .await?;
        insert_lens(
            db,
            "Mamiya",
            "Sekor E 200mm f/4",
            "Mamiya Z",
            Some("200"),
            Some("4"),
        )
        .await?;

        // Contax/Yashica (3)
        insert_lens(
            db,
            "Carl Zeiss",
            "Planar 50mm f/1.4",
            "Contax/Yashica",
            Some("50"),
            Some("1.4"),
        )
        .await?;
        insert_lens(
            db,
            "Carl Zeiss",
            "Tessar 45mm f/2.8",
            "Contax/Yashica",
            Some("45"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Carl Zeiss",
            "Planar 85mm f/1.4",
            "Contax/Yashica",
            Some("85"),
            Some("1.4"),
        )
        .await?;

        // Contax RF (2)
        insert_lens(
            db,
            "Carl Zeiss",
            "Sonnar 50mm f/1.5",
            "Contax RF",
            Some("50"),
            Some("1.5"),
        )
        .await?;
        insert_lens(
            db,
            "Carl Zeiss",
            "Biogon 35mm f/2.8",
            "Contax RF",
            Some("35"),
            Some("2.8"),
        )
        .await?;

        // Contax G (4)
        insert_lens(
            db,
            "Carl Zeiss",
            "Biogon 28mm f/2.8",
            "Contax G",
            Some("28"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Carl Zeiss",
            "Sonnar 90mm f/2.8",
            "Contax G",
            Some("90"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Carl Zeiss",
            "Planar 45mm f/2",
            "Contax G",
            Some("45"),
            Some("2"),
        )
        .await?;
        insert_lens(
            db,
            "Carl Zeiss",
            "Biogon 21mm f/2.8",
            "Contax G",
            Some("21"),
            Some("2.8"),
        )
        .await?;

        // Leica R (4)
        insert_lens(
            db,
            "Leica",
            "Elmarit-R 90mm f/2.8",
            "Leica R",
            Some("90"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Leica",
            "Summicron-R 50mm f/2",
            "Leica R",
            Some("50"),
            Some("2"),
        )
        .await?;
        insert_lens(
            db,
            "Leica",
            "Elmarit-R 35mm f/2.8",
            "Leica R",
            Some("35"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Leica",
            "Macro-Elmarit-R 60mm f/2.8",
            "Leica R",
            Some("60"),
            Some("2.8"),
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
