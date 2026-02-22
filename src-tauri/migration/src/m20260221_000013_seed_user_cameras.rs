use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

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
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Nikon', 'FE', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'SLR', datetime('now'), datetime('now')),
                ('Nikon', 'F2SB Photomic', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'SLR', datetime('now'), datetime('now')),
                ('Nikon', 'N80', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'SLR', datetime('now'), datetime('now')),
                ('Nikon', 'N75', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'SLR', datetime('now'), datetime('now')),
                ('Nikon', 'N90', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'SLR', datetime('now'), datetime('now')),
                ('Nikon', 'EM', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'SLR', datetime('now'), datetime('now')),
                ('Nikon', 'Nikkormat FT', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Nikon F'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // Minolta MD/MC mount (5 cameras)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Minolta', 'SR-T 101', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'SLR', datetime('now'), datetime('now')),
                ('Minolta', 'XD-11', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'SLR', datetime('now'), datetime('now')),
                ('Minolta', 'XD-7', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'SLR', datetime('now'), datetime('now')),
                ('Minolta', 'XE-1', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'SLR', datetime('now'), datetime('now')),
                ('Minolta', 'XG-M', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Minolta MD/MC'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // Olympus OM mount (1 camera)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Olympus', 'OM-1n', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Olympus OM'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // Pentax K mount (1 camera)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Pentax', 'K1000', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Pentax K'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // M42 (Universal) mount (5 cameras)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Asahi Pentax', 'SV', '35mm', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'SLR', datetime('now'), datetime('now')),
                ('Pentax', 'Spotmatic SP', '35mm', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'SLR', datetime('now'), datetime('now')),
                ('Fujica', 'ST705', '35mm', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'SLR', datetime('now'), datetime('now')),
                ('Fujica', 'ST901', '35mm', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'SLR', datetime('now'), datetime('now')),
                ('Voigtländer', 'VSL 1 (TM)', '35mm', (SELECT id FROM lens_mounts WHERE name = 'M42 (Universal)'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // Contax/Yashica mount (4 cameras)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Contax', 'AX', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Contax/Yashica'), 'SLR', datetime('now'), datetime('now')),
                ('Contax', 'RTSIII', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Contax/Yashica'), 'SLR', datetime('now'), datetime('now')),
                ('Contax', '139 Quartz', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Contax/Yashica'), 'SLR', datetime('now'), datetime('now')),
                ('Contax', 'S2', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Contax/Yashica'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // Leica R mount (2 cameras)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Leica', 'R6', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Leica R'), 'SLR', datetime('now'), datetime('now')),
                ('Leica', 'Leicaflex SL', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Leica R'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // Mamiya Z mount (2 cameras) — NEW mount
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Mamiya', 'ZM', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Mamiya Z'), 'SLR', datetime('now'), datetime('now')),
                ('Mamiya', 'ZE', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Mamiya Z'), 'SLR', datetime('now'), datetime('now'))",
        )
        .await?;

        // Rangefinders with new mounts
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Zorki', 'C', '35mm', (SELECT id FROM lens_mounts WHERE name = 'M39 (LTM)'), 'rangefinder', datetime('now'), datetime('now')),
                ('Contax', 'IIa', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Contax RF'), 'rangefinder', datetime('now'), datetime('now')),
                ('Contax', 'G1', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Contax G'), 'rangefinder', datetime('now'), datetime('now'))",
        )
        .await?;

        // Medium and large format
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Mamiya', 'RB67 Pro SD', '6x7', (SELECT id FROM lens_mounts WHERE name = 'Mamiya RB/RZ67'), 'SLR', datetime('now'), datetime('now')),
                ('Intrepid', '4x5 Black Edition', '4x5', (SELECT id FROM lens_mounts WHERE name = 'Copal #0'), 'view', datetime('now'), datetime('now'))",
        )
        .await?;

        // ── Phase 3: Fixed-lens cameras ───────────────────────────
        // Each fixed-lens camera needs: camera INSERT, lens INSERT,
        // junction INSERT, and default_lens_id UPDATE.

        // 33. Meopta Flexaret III (6x6 TLR)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Meopta', 'Flexaret III', '6x6', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'TLR', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Meopta', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Belar 80mm f/3.5', '80', '3.5', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Meopta' AND model = 'Flexaret III'),
                 (SELECT id FROM lenses WHERE brand = 'Meopta' AND name_on_lens = 'Belar 80mm f/3.5'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Meopta' AND name_on_lens = 'Belar 80mm f/3.5')
             WHERE brand = 'Meopta' AND model = 'Flexaret III'",
        ).await?;

        // 34. Voigtländer Brilliant (6x6 TLR)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Voigtländer', 'Brilliant', '6x6', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'TLR', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Voigtländer', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Voigtar 75mm f/7.7', '75', '7.7', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Voigtländer' AND model = 'Brilliant'),
                 (SELECT id FROM lenses WHERE brand = 'Voigtländer' AND name_on_lens = 'Voigtar 75mm f/7.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Voigtländer' AND name_on_lens = 'Voigtar 75mm f/7.7')
             WHERE brand = 'Voigtländer' AND model = 'Brilliant'",
        ).await?;

        // 35. Yashica Electro 35 GSN (35mm rangefinder)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Yashica', 'Electro 35 GSN', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'rangefinder', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Yashica', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Yashinon-DX 45mm f/1.7', '45', '1.7', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Yashica' AND model = 'Electro 35 GSN'),
                 (SELECT id FROM lenses WHERE brand = 'Yashica' AND name_on_lens = 'Yashinon-DX 45mm f/1.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Yashica' AND name_on_lens = 'Yashinon-DX 45mm f/1.7')
             WHERE brand = 'Yashica' AND model = 'Electro 35 GSN'",
        ).await?;

        // 36. Kodak Chevron (6x6 rangefinder)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Kodak', 'Chevron', '6x6', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'rangefinder', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Kodak', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Ektar 78mm f/3.5', '78', '3.5', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Kodak' AND model = 'Chevron'),
                 (SELECT id FROM lenses WHERE brand = 'Kodak' AND name_on_lens = 'Ektar 78mm f/3.5'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Kodak' AND name_on_lens = 'Ektar 78mm f/3.5')
             WHERE brand = 'Kodak' AND model = 'Chevron'",
        ).await?;

        // 37. Kodak No. 2 Brownie, Model D (6x9 box — simple meniscus lens)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Kodak', 'No. 2 Brownie, Model D', '6x9', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'box', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, created_at, updated_at) VALUES
                ('Kodak', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Meniscus', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Kodak' AND model = 'No. 2 Brownie, Model D'),
                 (SELECT id FROM lenses WHERE brand = 'Kodak' AND name_on_lens = 'Meniscus'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Kodak' AND name_on_lens = 'Meniscus')
             WHERE brand = 'Kodak' AND model = 'No. 2 Brownie, Model D'",
        ).await?;

        // 38. Rollei XF 35 (35mm rangefinder)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Rollei', 'XF 35', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'rangefinder', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Rollei', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Sonnar 40mm f/2.3', '40', '2.3', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Rollei' AND model = 'XF 35'),
                 (SELECT id FROM lenses WHERE brand = 'Rollei' AND name_on_lens = 'Sonnar 40mm f/2.3'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Rollei' AND name_on_lens = 'Sonnar 40mm f/2.3')
             WHERE brand = 'Rollei' AND model = 'XF 35'",
        ).await?;

        // 39. Nikon 35Ti (35mm point-and-shoot)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Nikon', '35Ti', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'point-and-shoot', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Nikon', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Nikkor 35mm f/2.8', '35', '2.8', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Nikon' AND model = '35Ti'),
                 (SELECT id FROM lenses WHERE brand = 'Nikon' AND name_on_lens = 'Nikkor 35mm f/2.8'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Nikon' AND name_on_lens = 'Nikkor 35mm f/2.8')
             WHERE brand = 'Nikon' AND model = '35Ti'",
        ).await?;

        // 40. Olympus Stylus Epic Zoom 80 (35mm point-and-shoot)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Olympus', 'Stylus Epic Zoom 80', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'point-and-shoot', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Olympus', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), '38-80mm f/4.5', '38-80', '4.5', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Olympus' AND model = 'Stylus Epic Zoom 80'),
                 (SELECT id FROM lenses WHERE brand = 'Olympus' AND name_on_lens = '38-80mm f/4.5'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Olympus' AND name_on_lens = '38-80mm f/4.5')
             WHERE brand = 'Olympus' AND model = 'Stylus Epic Zoom 80'",
        ).await?;

        // 41. Olympus 35RD (35mm rangefinder)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Olympus', '35RD', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'rangefinder', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Olympus', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'F.Zuiko 40mm f/1.7', '40', '1.7', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Olympus' AND model = '35RD'),
                 (SELECT id FROM lenses WHERE brand = 'Olympus' AND name_on_lens = 'F.Zuiko 40mm f/1.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Olympus' AND name_on_lens = 'F.Zuiko 40mm f/1.7')
             WHERE brand = 'Olympus' AND model = '35RD'",
        ).await?;

        // 42. Minolta Hi-Matic F (35mm rangefinder)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Minolta', 'Hi-Matic F', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'rangefinder', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Minolta', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Rokkor 38mm f/2.7', '38', '2.7', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Minolta' AND model = 'Hi-Matic F'),
                 (SELECT id FROM lenses WHERE brand = 'Minolta' AND name_on_lens = 'Rokkor 38mm f/2.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Minolta' AND name_on_lens = 'Rokkor 38mm f/2.7')
             WHERE brand = 'Minolta' AND model = 'Hi-Matic F'",
        ).await?;

        // 43. Canon Canonet G-III QL (35mm rangefinder, QL17 version)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Canon', 'Canonet G-III QL', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'rangefinder', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Canon', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Canon Lens 40mm f/1.7', '40', '1.7', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Canon' AND model = 'Canonet G-III QL'),
                 (SELECT id FROM lenses WHERE brand = 'Canon' AND name_on_lens = 'Canon Lens 40mm f/1.7'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = 'Canon' AND name_on_lens = 'Canon Lens 40mm f/1.7')
             WHERE brand = 'Canon' AND model = 'Canonet G-III QL'",
        ).await?;

        // 44. Zeiss Ikon S310 (35mm point-and-shoot)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Zeiss Ikon', 'S310', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'point-and-shoot', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Zeiss Ikon', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Tessar 50mm f/2.8', '50', '2.8', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Zeiss Ikon' AND model = 'S310'),
                 (SELECT MAX(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT MAX(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8')
             WHERE brand = 'Zeiss Ikon' AND model = 'S310'",
        ).await?;

        // 45. Zeiss Ikon Contessamat SBE (35mm point-and-shoot)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Zeiss Ikon', 'Contessamat SBE', '35mm', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'point-and-shoot', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Zeiss Ikon', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'Tessar 50mm f/2.8', '50', '2.8', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Zeiss Ikon' AND model = 'Contessamat SBE'),
                 (SELECT MAX(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8'))",
        ).await?;
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = (SELECT MAX(id) FROM lenses WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8')
             WHERE brand = 'Zeiss Ikon' AND model = 'Contessamat SBE'",
        ).await?;

        // 46. Diana F (medium format box)
        db.execute_unprepared(
            "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at) VALUES
                ('Diana', 'F', 'medium format', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), 'box', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at) VALUES
                ('Diana', (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens'), '75mm f/11', '75', '11', datetime('now'), datetime('now'))",
        ).await?;
        db.execute_unprepared(
            "INSERT INTO camera_lenses (camera_id, lens_id) VALUES
                ((SELECT id FROM cameras WHERE brand = 'Diana' AND model = 'F'),
                 (SELECT id FROM lenses WHERE brand = 'Diana' AND name_on_lens = '75mm f/11'))",
        ).await?;
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
