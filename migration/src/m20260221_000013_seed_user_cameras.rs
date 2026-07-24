use crate::seed_guard::{insert_camera, insert_lens, insert_lens_ordinal};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Re-run safety (kammerz-vlyu.8): `execute_unprepared` auto-commits per
        // statement and this migration is only recorded after `up()` returns, so a
        // mid-migration crash re-runs from statement 1 — duplicating already-
        // committed cameras/lenses (neither table has a unique key). Every insert is
        // guarded on its pre-m017 natural key via `crate::seed_guard`, and junction
        // inserts are `INSERT OR IGNORE` (composite PK). On a clean first run every
        // guard is true so all inserts fire in original order with the same
        // AUTOINCREMENT ids (byte-identical). See the module docs for the Zeiss
        // Tessar special case.

        // ── Phase 1: New lens mounts ──────────────────────────────
        db.execute_unprepared(
            "INSERT OR IGNORE INTO lens_mounts (name, created_at, updated_at) VALUES
                ('M39 (LTM)', datetime('now'), datetime('now')),
                ('Contax RF', datetime('now'), datetime('now')),
                ('Contax G', datetime('now'), datetime('now')),
                ('Mamiya Z', datetime('now'), datetime('now'))",
        )
        .await?;

        // ── Phase 2: Interchangeable-lens cameras ─────────────────
        // Nikon F mount (7 cameras)
        insert_camera(db, "Nikon", "FE", "35mm", "Nikon F", "SLR").await?;
        insert_camera(db, "Nikon", "F2SB Photomic", "35mm", "Nikon F", "SLR").await?;
        insert_camera(db, "Nikon", "N80", "35mm", "Nikon F", "SLR").await?;
        insert_camera(db, "Nikon", "N75", "35mm", "Nikon F", "SLR").await?;
        insert_camera(db, "Nikon", "N90", "35mm", "Nikon F", "SLR").await?;
        insert_camera(db, "Nikon", "EM", "35mm", "Nikon F", "SLR").await?;
        insert_camera(db, "Nikon", "Nikkormat FT", "35mm", "Nikon F", "SLR").await?;

        // Minolta MD/MC mount (5 cameras)
        insert_camera(db, "Minolta", "SR-T 101", "35mm", "Minolta MD/MC", "SLR").await?;
        insert_camera(db, "Minolta", "XD-11", "35mm", "Minolta MD/MC", "SLR").await?;
        insert_camera(db, "Minolta", "XD-7", "35mm", "Minolta MD/MC", "SLR").await?;
        insert_camera(db, "Minolta", "XE-1", "35mm", "Minolta MD/MC", "SLR").await?;
        insert_camera(db, "Minolta", "XG-M", "35mm", "Minolta MD/MC", "SLR").await?;

        // Olympus OM mount (1 camera)
        insert_camera(db, "Olympus", "OM-1n", "35mm", "Olympus OM", "SLR").await?;

        // Pentax K mount (1 camera)
        insert_camera(db, "Pentax", "K1000", "35mm", "Pentax K", "SLR").await?;

        // M42 (Universal) mount (5 cameras)
        insert_camera(db, "Asahi Pentax", "SV", "35mm", "M42 (Universal)", "SLR").await?;
        insert_camera(
            db,
            "Pentax",
            "Spotmatic SP",
            "35mm",
            "M42 (Universal)",
            "SLR",
        )
        .await?;
        insert_camera(db, "Fujica", "ST705", "35mm", "M42 (Universal)", "SLR").await?;
        insert_camera(db, "Fujica", "ST901", "35mm", "M42 (Universal)", "SLR").await?;
        insert_camera(
            db,
            "Voigtländer",
            "VSL 1 (TM)",
            "35mm",
            "M42 (Universal)",
            "SLR",
        )
        .await?;

        // Contax/Yashica mount (4 cameras)
        insert_camera(db, "Contax", "AX", "35mm", "Contax/Yashica", "SLR").await?;
        insert_camera(db, "Contax", "RTSIII", "35mm", "Contax/Yashica", "SLR").await?;
        insert_camera(db, "Contax", "139 Quartz", "35mm", "Contax/Yashica", "SLR").await?;
        insert_camera(db, "Contax", "S2", "35mm", "Contax/Yashica", "SLR").await?;

        // Leica R mount (2 cameras)
        insert_camera(db, "Leica", "R6", "35mm", "Leica R", "SLR").await?;
        insert_camera(db, "Leica", "Leicaflex SL", "35mm", "Leica R", "SLR").await?;

        // Mamiya Z mount (2 cameras) — NEW mount
        insert_camera(db, "Mamiya", "ZM", "35mm", "Mamiya Z", "SLR").await?;
        insert_camera(db, "Mamiya", "ZE", "35mm", "Mamiya Z", "SLR").await?;

        // Rangefinders with new mounts
        insert_camera(db, "Zorki", "C", "35mm", "M39 (LTM)", "rangefinder").await?;
        insert_camera(db, "Contax", "IIa", "35mm", "Contax RF", "rangefinder").await?;
        insert_camera(db, "Contax", "G1", "35mm", "Contax G", "rangefinder").await?;

        // Medium and large format
        insert_camera(db, "Mamiya", "RB67 Pro SD", "6x7", "Mamiya RB/RZ67", "SLR").await?;
        insert_camera(
            db,
            "Intrepid",
            "4x5 Black Edition",
            "4x5",
            "Copal #0",
            "view",
        )
        .await?;

        // ── Phase 3: Fixed-lens cameras ───────────────────────────
        // Each fixed-lens camera needs: camera INSERT, lens INSERT, junction
        // INSERT OR IGNORE, and default_lens_id UPDATE. Lens natural key is the
        // pre-m017 `name_on_lens` column.

        // 33. Meopta Flexaret III (6x6 TLR)
        insert_camera(db, "Meopta", "Flexaret III", "6x6", "Fixed Lens", "TLR").await?;
        insert_lens(
            db,
            "Meopta",
            "Belar 80mm f/3.5",
            "Fixed Lens",
            Some("80"),
            Some("3.5"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Meopta' AND model = 'Flexaret III'),
                 (SELECT id FROM lenses WHERE brand = 'Meopta' AND name_on_lens = 'Belar 80mm f/3.5'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Meopta' AND name_on_lens = 'Belar 80mm f/3.5')
             WHERE brand = 'Meopta' AND model = 'Flexaret III'",
        ).await?;

        // 34. Voigtländer Brilliant (6x6 TLR)
        insert_camera(db, "Voigtländer", "Brilliant", "6x6", "Fixed Lens", "TLR").await?;
        insert_lens(
            db,
            "Voigtländer",
            "Voigtar 75mm f/7.7",
            "Fixed Lens",
            Some("75"),
            Some("7.7"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Voigtländer' AND model = 'Brilliant'),
                 (SELECT id FROM lenses WHERE brand = 'Voigtländer' AND name_on_lens = 'Voigtar 75mm f/7.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Voigtländer' AND name_on_lens = 'Voigtar 75mm f/7.7')
             WHERE brand = 'Voigtländer' AND model = 'Brilliant'",
        ).await?;

        // 35. Yashica Electro 35 GSN (35mm rangefinder)
        insert_camera(
            db,
            "Yashica",
            "Electro 35 GSN",
            "35mm",
            "Fixed Lens",
            "rangefinder",
        )
        .await?;
        insert_lens(
            db,
            "Yashica",
            "Yashinon-DX 45mm f/1.7",
            "Fixed Lens",
            Some("45"),
            Some("1.7"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Yashica' AND model = 'Electro 35 GSN'),
                 (SELECT id FROM lenses WHERE brand = 'Yashica' AND name_on_lens = 'Yashinon-DX 45mm f/1.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Yashica' AND name_on_lens = 'Yashinon-DX 45mm f/1.7')
             WHERE brand = 'Yashica' AND model = 'Electro 35 GSN'",
        ).await?;

        // 36. Kodak Chevron (6x6 rangefinder)
        insert_camera(db, "Kodak", "Chevron", "6x6", "Fixed Lens", "rangefinder").await?;
        insert_lens(
            db,
            "Kodak",
            "Ektar 78mm f/3.5",
            "Fixed Lens",
            Some("78"),
            Some("3.5"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Kodak' AND model = 'Chevron'),
                 (SELECT id FROM lenses WHERE brand = 'Kodak' AND name_on_lens = 'Ektar 78mm f/3.5'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Kodak' AND name_on_lens = 'Ektar 78mm f/3.5')
             WHERE brand = 'Kodak' AND model = 'Chevron'",
        ).await?;

        // 37. Kodak No. 2 Brownie, Model D (6x9 box — simple meniscus lens)
        insert_camera(
            db,
            "Kodak",
            "No. 2 Brownie, Model D",
            "6x9",
            "Fixed Lens",
            "box",
        )
        .await?;
        insert_lens(db, "Kodak", "Meniscus", "Fixed Lens", None, None).await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Kodak' AND model = 'No. 2 Brownie, Model D'),
                 (SELECT id FROM lenses WHERE brand = 'Kodak' AND name_on_lens = 'Meniscus'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Kodak' AND name_on_lens = 'Meniscus')
             WHERE brand = 'Kodak' AND model = 'No. 2 Brownie, Model D'",
        ).await?;

        // 38. Rollei XF 35 (35mm rangefinder)
        insert_camera(db, "Rollei", "XF 35", "35mm", "Fixed Lens", "rangefinder").await?;
        insert_lens(
            db,
            "Rollei",
            "Sonnar 40mm f/2.3",
            "Fixed Lens",
            Some("40"),
            Some("2.3"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Rollei' AND model = 'XF 35'),
                 (SELECT id FROM lenses WHERE brand = 'Rollei' AND name_on_lens = 'Sonnar 40mm f/2.3'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Rollei' AND name_on_lens = 'Sonnar 40mm f/2.3')
             WHERE brand = 'Rollei' AND model = 'XF 35'",
        ).await?;

        // 39. Nikon 35Ti (35mm point-and-shoot)
        insert_camera(db, "Nikon", "35Ti", "35mm", "Fixed Lens", "point-and-shoot").await?;
        insert_lens(
            db,
            "Nikon",
            "Nikkor 35mm f/2.8",
            "Fixed Lens",
            Some("35"),
            Some("2.8"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Nikon' AND model = '35Ti'),
                 (SELECT id FROM lenses WHERE brand = 'Nikon' AND name_on_lens = 'Nikkor 35mm f/2.8'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Nikon' AND name_on_lens = 'Nikkor 35mm f/2.8')
             WHERE brand = 'Nikon' AND model = '35Ti'",
        ).await?;

        // 40. Olympus Stylus Epic Zoom 80 (35mm point-and-shoot)
        insert_camera(
            db,
            "Olympus",
            "Stylus Epic Zoom 80",
            "35mm",
            "Fixed Lens",
            "point-and-shoot",
        )
        .await?;
        insert_lens(
            db,
            "Olympus",
            "38-80mm f/4.5",
            "Fixed Lens",
            Some("38-80"),
            Some("4.5"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Olympus' AND model = 'Stylus Epic Zoom 80'),
                 (SELECT id FROM lenses WHERE brand = 'Olympus' AND name_on_lens = '38-80mm f/4.5'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Olympus' AND name_on_lens = '38-80mm f/4.5')
             WHERE brand = 'Olympus' AND model = 'Stylus Epic Zoom 80'",
        ).await?;

        // 41. Olympus 35RD (35mm rangefinder)
        insert_camera(db, "Olympus", "35RD", "35mm", "Fixed Lens", "rangefinder").await?;
        insert_lens(
            db,
            "Olympus",
            "F.Zuiko 40mm f/1.7",
            "Fixed Lens",
            Some("40"),
            Some("1.7"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Olympus' AND model = '35RD'),
                 (SELECT id FROM lenses WHERE brand = 'Olympus' AND name_on_lens = 'F.Zuiko 40mm f/1.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Olympus' AND name_on_lens = 'F.Zuiko 40mm f/1.7')
             WHERE brand = 'Olympus' AND model = '35RD'",
        ).await?;

        // 42. Minolta Hi-Matic F (35mm rangefinder)
        insert_camera(
            db,
            "Minolta",
            "Hi-Matic F",
            "35mm",
            "Fixed Lens",
            "rangefinder",
        )
        .await?;
        insert_lens(
            db,
            "Minolta",
            "Rokkor 38mm f/2.7",
            "Fixed Lens",
            Some("38"),
            Some("2.7"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Minolta' AND model = 'Hi-Matic F'),
                 (SELECT id FROM lenses WHERE brand = 'Minolta' AND name_on_lens = 'Rokkor 38mm f/2.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Minolta' AND name_on_lens = 'Rokkor 38mm f/2.7')
             WHERE brand = 'Minolta' AND model = 'Hi-Matic F'",
        ).await?;

        // 43. Canon Canonet G-III QL (35mm rangefinder, QL17 version)
        insert_camera(
            db,
            "Canon",
            "Canonet G-III QL",
            "35mm",
            "Fixed Lens",
            "rangefinder",
        )
        .await?;
        insert_lens(
            db,
            "Canon",
            "Canon Lens 40mm f/1.7",
            "Fixed Lens",
            Some("40"),
            Some("1.7"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Canon' AND model = 'Canonet G-III QL'),
                 (SELECT id FROM lenses WHERE brand = 'Canon' AND name_on_lens = 'Canon Lens 40mm f/1.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Canon' AND name_on_lens = 'Canon Lens 40mm f/1.7')
             WHERE brand = 'Canon' AND model = 'Canonet G-III QL'",
        ).await?;

        // 44. Zeiss Ikon S310 (35mm point-and-shoot)
        // SPECIAL CASE: this and the Contessamat below share the lens natural key
        // ('Zeiss Ikon','Tessar 50mm f/2.8','Fixed Lens') — two intentional rows.
        // The ordinal guard keeps exactly two (S310 = the first / lower id).
        //
        // DO NOT change these two SELECTs back to MAX(id). S310's junction + default
        // resolve to MIN(id) — the lower/first Tessar, which is S310's own lens. On a
        // clean first run only one Tessar exists at this point, so MIN == MAX (byte-
        // identical to the original). But on a crash RE-RUN both Tessars already
        // exist, and MAX(id) would then resolve S310 to the Contessamat's higher-id
        // lens: a spurious second S310 junction row (INSERT OR IGNORE won't dedup a
        // different (camera_id, lens_id) pair) plus a wrong non-null default that
        // NEITHER m019 (FK-off, so its rebuild doesn't cascade-wipe) NOR m020 (heals
        // only NULL defaults) repairs. MIN(id) is re-run-safe and matches m020's own
        // split (S310 -> MIN, Contessamat -> MAX). See seed_migrations_idempotent.
        insert_camera(
            db,
            "Zeiss Ikon",
            "S310",
            "35mm",
            "Fixed Lens",
            "point-and-shoot",
        )
        .await?;
        insert_lens_ordinal(
            db,
            "Zeiss Ikon",
            "Tessar 50mm f/2.8",
            "Fixed Lens",
            Some("50"),
            Some("2.8"),
            1,
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Zeiss Ikon' AND model = 'S310'),
                 (SELECT MIN(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT MIN(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8')
             WHERE brand = 'Zeiss Ikon' AND model = 'S310'",
        ).await?;

        // 45. Zeiss Ikon Contessamat SBE (35mm point-and-shoot)
        // Second row of the intentional Tessar pair (ordinal guard keeps <2). The
        // junction + default resolve to MAX(id) — always the Contessamat's own
        // (higher id) lens, re-run-safe as-is.
        insert_camera(
            db,
            "Zeiss Ikon",
            "Contessamat SBE",
            "35mm",
            "Fixed Lens",
            "point-and-shoot",
        )
        .await?;
        insert_lens_ordinal(
            db,
            "Zeiss Ikon",
            "Tessar 50mm f/2.8",
            "Fixed Lens",
            Some("50"),
            Some("2.8"),
            2,
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Zeiss Ikon' AND model = 'Contessamat SBE'),
                 (SELECT MAX(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT MAX(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8')
             WHERE brand = 'Zeiss Ikon' AND model = 'Contessamat SBE'",
        ).await?;

        // 46. Diana F (medium format box)
        insert_camera(db, "Diana", "F", "medium format", "Fixed Lens", "box").await?;
        insert_lens(
            db,
            "Diana",
            "75mm f/11",
            "Fixed Lens",
            Some("75"),
            Some("11"),
        )
        .await?;
        db.execute_unprepared(
            "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Diana' AND model = 'F'),
                 (SELECT id FROM lenses WHERE brand = 'Diana' AND name_on_lens = '75mm f/11'))",
        )
        .await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Diana' AND name_on_lens = '75mm f/11')
             WHERE brand = 'Diana' AND model = 'F'",
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Null out default_lens_id for all seeded cameras before deleting lenses
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = NULL WHERE brand || ' ' || model IN (
                'Meopta Flexaret III', 'Voigtländer Brilliant',
                'Yashica Electro 35 GSN', 'Kodak Chevron',
                'Kodak No. 2 Brownie, Model D', 'Rollei XF 35',
                'Nikon 35Ti', 'Olympus Stylus Epic Zoom 80',
                'Olympus 35RD', 'Minolta Hi-Matic F',
                'Canon Canonet G-III QL', 'Zeiss Ikon S310',
                'Zeiss Ikon Contessamat SBE', 'Diana F'
            )",
        )
        .await?;

        // Delete junction entries for fixed-lens cameras
        db.execute_unprepared(
            "DELETE FROM camera_lenses WHERE camera_id IN (
                SELECT id FROM cameras WHERE brand || ' ' || model IN (
                    'Meopta Flexaret III', 'Voigtländer Brilliant',
                    'Yashica Electro 35 GSN', 'Kodak Chevron',
                    'Kodak No. 2 Brownie, Model D', 'Rollei XF 35',
                    'Nikon 35Ti', 'Olympus Stylus Epic Zoom 80',
                    'Olympus 35RD', 'Minolta Hi-Matic F',
                    'Canon Canonet G-III QL', 'Zeiss Ikon S310',
                    'Zeiss Ikon Contessamat SBE', 'Diana F'
                )
            )",
        )
        .await?;

        // Delete lenses for fixed-lens cameras (they all use the Fixed Lens mount)
        db.execute_unprepared(
            "DELETE FROM lenses WHERE lens_mount_id = (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens')
             AND brand || ' ' || COALESCE(name_on_lens, '') IN (
                'Meopta Belar 80mm f/3.5', 'Voigtländer Voigtar 75mm f/7.7',
                'Yashica Yashinon-DX 45mm f/1.7', 'Kodak Ektar 78mm f/3.5',
                'Kodak Meniscus',
                'Rollei Sonnar 40mm f/2.3', 'Nikon Nikkor 35mm f/2.8',
                'Olympus 38-80mm f/4.5', 'Olympus F.Zuiko 40mm f/1.7',
                'Minolta Rokkor 38mm f/2.7', 'Canon Canon Lens 40mm f/1.7',
                'Zeiss Ikon Tessar 50mm f/2.8', 'Diana 75mm f/11'
             )",
        )
        .await?;

        // Delete all seeded cameras
        db.execute_unprepared(
            "DELETE FROM cameras WHERE brand || ' ' || model IN (
                'Nikon FE', 'Nikon F2SB Photomic', 'Nikon N80', 'Nikon N75',
                'Nikon N90', 'Nikon EM', 'Nikon Nikkormat FT',
                'Minolta SR-T 101', 'Minolta XD-11', 'Minolta XD-7',
                'Minolta XE-1', 'Minolta XG-M',
                'Olympus OM-1n', 'Pentax K1000', 'Asahi Pentax SV',
                'Pentax Spotmatic SP', 'Fujica ST705', 'Fujica ST901',
                'Voigtländer VSL 1 (TM)',
                'Contax AX', 'Contax RTSIII', 'Contax 139 Quartz', 'Contax S2',
                'Leica R6', 'Leica Leicaflex SL',
                'Mamiya ZM', 'Mamiya ZE',
                'Zorki C', 'Contax IIa', 'Contax G1',
                'Mamiya RB67 Pro SD', 'Intrepid 4x5 Black Edition',
                'Meopta Flexaret III', 'Voigtländer Brilliant',
                'Yashica Electro 35 GSN', 'Kodak Chevron',
                'Kodak No. 2 Brownie, Model D', 'Rollei XF 35',
                'Nikon 35Ti', 'Olympus Stylus Epic Zoom 80',
                'Olympus 35RD', 'Minolta Hi-Matic F',
                'Canon Canonet G-III QL', 'Zeiss Ikon S310',
                'Zeiss Ikon Contessamat SBE', 'Diana F'
            )",
        )
        .await?;

        // Delete new lens mounts
        db.execute_unprepared(
            "DELETE FROM lens_mounts WHERE name IN ('M39 (LTM)', 'Contax RF', 'Contax G', 'Mamiya Z')",
        )
        .await?;

        Ok(())
    }
}
