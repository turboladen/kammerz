use crate::seed_guard::insert_lens;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── Seed 20 interchangeable lenses ──────────────────────────
        // Guarded on the pre-m017 natural key (brand, name_on_lens, lens_mount_id)
        // so a mid-migration crash re-run doesn't duplicate them (kammerz-vlyu.8).
        // Grouped by mount system for readability; order preserves ids on a clean run.

        // Pentax K (1)
        insert_lens(
            db,
            "Asahi",
            "SMC Pentax 55mm f/2",
            "Pentax K",
            Some("55"),
            Some("2"),
        )
        .await?;

        // M42 Universal (4)
        insert_lens(
            db,
            "Asahi",
            "SMC Super Takumar 50mm f/1.4",
            "M42 (Universal)",
            Some("50"),
            Some("1.4"),
        )
        .await?;
        insert_lens(
            db,
            "Asahi",
            "Super-Multi-Coated TAKUMAR 35mm f/3.5",
            "M42 (Universal)",
            Some("35"),
            Some("3.5"),
        )
        .await?;
        insert_lens(
            db,
            "Asahi",
            "Super-Multi-Coated TAKUMAR 105mm f/2.8",
            "M42 (Universal)",
            Some("105"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Helios",
            "44-2 58mm f/2",
            "M42 (Universal)",
            Some("58"),
            Some("2"),
        )
        .await?;

        // M39 / LTM (1)
        insert_lens(
            db,
            "Industar",
            "N-61 L/D 55mm f/2.8",
            "M39 (LTM)",
            Some("55"),
            Some("2.8"),
        )
        .await?;

        // Mamiya 645 (1)
        insert_lens(
            db,
            "Mamiya",
            "Sekor C 80mm f/2.8 N",
            "Mamiya 645",
            Some("80"),
            Some("2.8"),
        )
        .await?;

        // Mamiya Z (1)
        insert_lens(
            db,
            "Mamiya",
            "Sekor EF 35mm f/2.8",
            "Mamiya Z",
            Some("35"),
            Some("2.8"),
        )
        .await?;

        // Minolta MD/MC (7)
        insert_lens(
            db,
            "Minolta",
            "MC Macro Rokkor QF 50mm f/3.5",
            "Minolta MD/MC",
            Some("50"),
            Some("3.5"),
        )
        .await?;
        insert_lens(
            db,
            "Minolta",
            "MC Rokkor-PF 55mm f/1.7",
            "Minolta MD/MC",
            Some("55"),
            Some("1.7"),
        )
        .await?;
        insert_lens(
            db,
            "Minolta",
            "MC Tele Rokkor-HF 30cm f/4.5",
            "Minolta MD/MC",
            Some("300"),
            Some("4.5"),
        )
        .await?;
        insert_lens(
            db,
            "Minolta",
            "MD Rokkor 50mm f/1.4",
            "Minolta MD/MC",
            Some("50"),
            Some("1.4"),
        )
        .await?;
        insert_lens(
            db,
            "Minolta",
            "MD Rokkor 50mm f/1.7",
            "Minolta MD/MC",
            Some("50"),
            Some("1.7"),
        )
        .await?;
        insert_lens(
            db,
            "Minolta",
            "MD Tele Rokkor-X 135mm f/2.8",
            "Minolta MD/MC",
            Some("135"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Minolta",
            "MD W.Rokkor 28mm f/2.8",
            "Minolta MD/MC",
            Some("28"),
            Some("2.8"),
        )
        .await?;

        // Nikon F (4)
        insert_lens(
            db,
            "Nikon",
            "50mm f/1.8 Series E",
            "Nikon F",
            Some("50"),
            Some("1.8"),
        )
        .await?;
        insert_lens(
            db,
            "Nikon",
            "Nikkor 50mm f/1.4 AF-D",
            "Nikon F",
            Some("50"),
            Some("1.4"),
        )
        .await?;
        insert_lens(
            db,
            "Nikon",
            "Nikkor 55mm f/1.2",
            "Nikon F",
            Some("55"),
            Some("1.2"),
        )
        .await?;
        insert_lens(
            db,
            "Nikon",
            "Nikkor 70-210mm f/4.0 AF-D",
            "Nikon F",
            Some("70-210"),
            Some("4"),
        )
        .await?;

        // Olympus OM (1)
        insert_lens(
            db,
            "Olympus",
            "F. Zuiko Auto-S 50mm f/1.8",
            "Olympus OM",
            Some("50"),
            Some("1.8"),
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
