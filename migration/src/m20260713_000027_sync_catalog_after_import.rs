//! Sync the seed data to the real catalog after the 2026-06 Apple Notes/Numbers
//! roll import (kammerz-1gi / kammerz-94b / kammerz-lxg).
//!
//! The import added rows straight into the live DB that the seed migrations never
//! knew about, so a freshly-migrated database did not reflect the true catalog.
//! This migration closes that gap in one unit:
//!   - 33 import-added film stocks (kammerz-1gi)
//!   - 15 import-added lenses (13 standalone here + 2 fixed-lens ones with their
//!     cameras below) (kammerz-94b)
//!   - 2 import-added cameras, a prefix set on the existing seeded cameras, and a
//!     Nikkormat FT->FTn model correction (kammerz-lxg)
//!
//! Idempotency / re-run safety: `execute_unprepared` auto-commits per statement,
//! so a mid-migration failure would leave 027 unrecorded and re-run from the top.
//! `film_stocks` has a UNIQUE(brand,name,format) index, so its inserts use
//! `INSERT OR IGNORE`. `cameras`/`lenses` have NO unique key, so every insert is
//! guarded with `WHERE NOT EXISTS` on the natural key. `camera_lenses` has a
//! composite PK, so its inserts use `INSERT OR IGNORE`. All FK ids are resolved
//! via name subqueries (never hardcoded ids — they vary across environments).
//!
//! This migration runs AFTER m..017 renamed `lenses.name_on_lens` -> `model`, so
//! lens inserts target the `model` column.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // ── kammerz-lxg: Nikkormat FT -> FTn correction ─────────────
        // Must run BEFORE the prefix set below, which keys the NFTN prefix on the
        // corrected model. Idempotent: a second run matches nothing.
        db.execute_unprepared(
            "UPDATE cameras SET model = 'Nikkormat FTn'
             WHERE brand = 'Nikon' AND model = 'Nikkormat FT'",
        )
        .await?;

        // ── kammerz-1gi: import-added film stocks (33) ──────────────
        // exposure_count omitted -> NULL for every row (matches the live catalog;
        // the 135/4x5 NULLs are a known import artifact deferred to kammerz-75e2).
        // Safe/idempotent via the UNIQUE(brand,name,format) index.
        db.execute_unprepared(
            "INSERT OR IGNORE INTO film_stocks (brand, name, format, stock_type, iso, created_at, updated_at) VALUES
                ('Agfa', 'APX 100', '135', 'bw-negative', 100, datetime('now'), datetime('now')),
                ('Agfa', 'APX 400', '135', 'bw-negative', 400, datetime('now'), datetime('now')),
                ('Agfa', 'Vista Plus 200', '135', 'color-negative', 200, datetime('now'), datetime('now')),
                ('ArsImago', '320', '120', 'bw-negative', 320, datetime('now'), datetime('now')),
                ('Bergger', 'Pancro 400', '135', 'bw-negative', 400, datetime('now'), datetime('now')),
                ('CatLabs', 'X Film 80', '4x5', 'bw-negative', 80, datetime('now'), datetime('now')),
                ('CineStill', '50D', '135', 'color-negative', 50, datetime('now'), datetime('now')),
                ('CineStill', '800T', '135', 'color-negative', 800, datetime('now'), datetime('now')),
                ('CineStill', 'BwXX', '135', 'bw-negative', 250, datetime('now'), datetime('now')),
                ('FPP', 'RetroChrome 400', '135', 'color-slide', 400, datetime('now'), datetime('now')),
                ('FPP', 'Tasma NK-II 100', '135', 'bw-negative', 100, datetime('now'), datetime('now')),
                ('Fujifilm', 'Acros 100', '135', 'bw-negative', 100, datetime('now'), datetime('now')),
                ('Fujifilm', 'Fujicolor 200', '135', 'color-negative', 200, datetime('now'), datetime('now')),
                ('Fujifilm', 'Natura 1600', '135', 'color-negative', 1600, datetime('now'), datetime('now')),
                ('Fujifilm', 'Neopan Acros 100 II', '135', 'bw-negative', 100, datetime('now'), datetime('now')),
                ('Fujifilm', 'Sensia 100', '135', 'color-slide', 100, datetime('now'), datetime('now')),
                ('Fujifilm', 'Superia 100', '135', 'color-negative', 100, datetime('now'), datetime('now')),
                ('Fujifilm', 'Superia 1600', '135', 'color-negative', 1600, datetime('now'), datetime('now')),
                ('Fujifilm', 'Superia 200', '135', 'color-negative', 200, datetime('now'), datetime('now')),
                ('Fujifilm', 'Superia 400', '135', 'color-negative', 400, datetime('now'), datetime('now')),
                ('Holga', '400 BW', '135', 'bw-negative', 400, datetime('now'), datetime('now')),
                ('Ilford', 'SFX 200', '135', 'bw-negative', 200, datetime('now'), datetime('now')),
                ('Kodak', 'Eastman Double-X 5222', '135', 'bw-negative', 250, datetime('now'), datetime('now')),
                ('Kodak', 'Gold 400', '135', 'color-negative', 400, datetime('now'), datetime('now')),
                ('Kodak', 'Gold 800', '135', 'color-negative', 800, datetime('now'), datetime('now')),
                ('Kodak', 'Hawkeye Super Color 200-400', '135', 'color-negative', 400, datetime('now'), datetime('now')),
                ('Kodak', 'Hawkeye Surveillance Film', '135', 'bw-negative', 200, datetime('now'), datetime('now')),
                ('Kodak', 'Kodacolor II', '120', 'color-negative', 100, datetime('now'), datetime('now')),
                ('Kodak', 'Max 400', '135', 'color-negative', 400, datetime('now'), datetime('now')),
                ('Kodak', 'Portra 160VC', '120', 'color-negative', 160, datetime('now'), datetime('now')),
                ('Kodak', 'ProImage 100', '135', 'color-negative', 100, datetime('now'), datetime('now')),
                ('Quality Photo', 'Best Choice Brand 200', '135', 'color-negative', 200, datetime('now'), datetime('now')),
                ('Rollei', 'CN 200', '135', 'color-negative', 200, datetime('now'), datetime('now'))",
        )
        .await?;

        // ── kammerz-94b: standalone import-added lenses (13) ────────
        // Each is a guarded INSERT ... SELECT ... WHERE NOT EXISTS on the natural
        // key (brand + model + mount). Mount names use the exact stored strings
        // (e.g. 'M42 (Universal)', not 'M42'). The Rodenstock is the only
        // import-added large-format lens; it is wired as the Intrepid's default
        // lens further below. The Olympus/Ansco fixed-lens lenses live with their
        // cameras in the next section.
        insert_lens(
            db,
            "Mamiya",
            "90mm K/L",
            "Mamiya RB/RZ67",
            Some("90"),
            Some("3.5"),
        )
        .await?;
        insert_lens(
            db,
            "Exakta",
            "28mm f/2.8 Macro",
            "Minolta MD/MC",
            Some("28"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Mamiya",
            "Sekor E 28mm f/2.8",
            "Mamiya Z",
            Some("28"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Mamiya",
            "645 Sekor C N 80mm f/2.8 (adapted)",
            "Mamiya Z",
            Some("80"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Mamiya",
            "Sekor E 135mm f/2.8",
            "Mamiya Z",
            Some("135"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Asahi",
            "Super-Takumar 85mm f/1.9 (M42, adapted)",
            "Mamiya Z",
            Some("85"),
            Some("1.9"),
        )
        .await?;
        insert_lens(
            db,
            "Asahi",
            "Super-Takumar 105mm f/2.8 (M42, adapted)",
            "Mamiya Z",
            Some("105"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Asahi",
            "SMC Takumar 200mm f/4",
            "M42 (Universal)",
            Some("200"),
            Some("4"),
        )
        .await?;
        insert_lens(
            db,
            "Tokina",
            "AT-X 116 PRO DX 11-16mm f/2.8 (borrowed)",
            "Nikon F",
            Some("11-16"),
            Some("2.8"),
        )
        .await?;
        insert_lens(
            db,
            "Asahi",
            "Super-Takumar 135mm f/3.5",
            "M42 (Universal)",
            Some("135"),
            Some("3.5"),
        )
        .await?;
        insert_lens(
            db,
            "Nikon",
            "Nikkor 50mm f/1.8 AF-D",
            "Nikon F",
            Some("50"),
            Some("1.8"),
        )
        .await?;
        insert_lens(
            db,
            "Rodenstock",
            "Sironar-N 240mm f/5.6 MC",
            "Compur #3",
            Some("240"),
            Some("5.6"),
        )
        .await?;
        insert_lens(
            db,
            "Tokina",
            "Zoom that came on the Fuji ST-901",
            "M42 (Universal)",
            None,
            None,
        )
        .await?;

        // ── kammerz-lxg + kammerz-94b: fixed-lens cameras (2) ───────
        // Each fixed-lens camera needs the canonical 4-statement quad (camera,
        // lens, junction, default_lens_id UPDATE), all guarded/idempotent.

        // Olympus Stylus Zoom 105 (35mm point-and-shoot). Its lens is seeded with
        // NORMALIZED values (bare max_aperture, no 'mm' on focal_length) — the
        // live row's 'f/4.5-8.9' / '38-105mm' is repaired below (kammerz-jd1).
        // Prefixes (OX/AJ) are set by the CASE UPDATE below alongside the existing
        // cameras, so they land on a pre-existing live row too — not just here.
        insert_camera(
            db,
            "Olympus",
            "Stylus Zoom 105",
            "35mm",
            "Fixed Lens",
            "point-and-shoot",
        )
        .await?;
        insert_lens(
            db,
            "Olympus",
            "Zoom 38-105mm",
            "Fixed Lens",
            Some("38-105"),
            Some("4.5-8.9"),
        )
        .await?;
        link_default_lens(db, "Olympus", "Stylus Zoom 105", "Olympus", "Zoom 38-105mm").await?;

        // Ansco Junior Model 1A (medium format box, simple meniscus lens).
        insert_camera(
            db,
            "Ansco",
            "Junior Model 1A",
            "medium format",
            "Fixed Lens",
            "box",
        )
        .await?;
        insert_lens(db, "Ansco", "Meniscus", "Fixed Lens", None, None).await?;
        link_default_lens(db, "Ansco", "Junior Model 1A", "Ansco", "Meniscus").await?;

        // ── kammerz-94b: Intrepid default-lens wiring ───────────────
        // The Intrepid 4x5 is already seeded (m..013); the import attached the
        // Rodenstock (inserted above) as its default lens. Wire the junction +
        // default here (guarded/idempotent). The Compur#3-lens / Copal#0-camera
        // mount mismatch is fine — large-format cross-mount compat, no constraint.
        link_default_lens(
            db,
            "Intrepid",
            "4x5 Black Edition",
            "Rodenstock",
            "Sironar-N 240mm f/5.6 MC",
        )
        .await?;

        // ── kammerz-jd1: normalize the one f/-prefixed live lens row ─
        // Olympus Zoom 38-105mm is the only lens catalog-wide with a display-
        // breaking f/ prefix / 'mm' suffix. On a fresh DB the seed above already
        // wrote the normalized values, so this UPDATE is a no-op (guarded by the
        // old value); on the live DB it fixes the pre-existing row.
        db.execute_unprepared(
            "UPDATE lenses SET max_aperture = '4.5-8.9', focal_length = '38-105'
             WHERE brand = 'Olympus' AND model = 'Zoom 38-105mm' AND max_aperture = 'f/4.5-8.9'",
        )
        .await?;

        // ── kammerz-lxg: prefixes on all seeded cameras ────────────
        // Single idempotent CASE UPDATE (ELSE prefix leaves untouched rows alone).
        // Covers all 41 prefixed cameras, INCLUDING the 2 new fixed-lens ones — so
        // a live catalog whose Olympus/Ansco rows predate this migration (where the
        // guarded insert_camera above no-ops) still gets OX/AJ set here, not only a
        // fresh DB. The Nikkormat rename ran first, so 'Nikkormat FTn' matches here.
        db.execute_unprepared(
            "UPDATE cameras SET prefix = CASE
                WHEN brand = 'Olympus' AND model = 'Stylus Zoom 105' THEN 'OX'
                WHEN brand = 'Ansco' AND model = 'Junior Model 1A' THEN 'AJ'
                WHEN brand = 'Nikon' AND model = 'FE' THEN 'NFE'
                WHEN brand = 'Nikon' AND model = 'F2SB Photomic' THEN 'NF2'
                WHEN brand = 'Nikon' AND model = 'N80' THEN 'N80'
                WHEN brand = 'Nikon' AND model = 'Nikkormat FTn' THEN 'NFTN'
                WHEN brand = 'Minolta' AND model = 'SR-T 101' THEN 'M101'
                WHEN brand = 'Minolta' AND model = 'XD-11' THEN 'MXD11'
                WHEN brand = 'Minolta' AND model = 'XD-7' THEN 'MXD7'
                WHEN brand = 'Minolta' AND model = 'XE-1' THEN 'MXE1'
                WHEN brand = 'Olympus' AND model = 'OM-1n' THEN 'OM1'
                WHEN brand = 'Pentax' AND model = 'K1000' THEN 'PKK'
                WHEN brand = 'Asahi Pentax' AND model = 'SV' THEN 'PSV'
                WHEN brand = 'Pentax' AND model = 'Spotmatic SP' THEN 'PSP'
                WHEN brand = 'Fujica' AND model = 'ST705' THEN 'F705'
                WHEN brand = 'Fujica' AND model = 'ST901' THEN 'F901'
                WHEN brand = 'Voigtländer' AND model = 'VSL 1 (TM)' THEN 'VSL1T'
                WHEN brand = 'Contax' AND model = 'AX' THEN 'CAX'
                WHEN brand = 'Contax' AND model = 'RTSIII' THEN 'CR3'
                WHEN brand = 'Contax' AND model = '139 Quartz' THEN 'C139'
                WHEN brand = 'Contax' AND model = 'S2' THEN 'CS2'
                WHEN brand = 'Leica' AND model = 'R6' THEN 'LR6'
                WHEN brand = 'Leica' AND model = 'Leicaflex SL' THEN 'LSL'
                WHEN brand = 'Mamiya' AND model = 'ZM' THEN 'MZM'
                WHEN brand = 'Zorki' AND model = 'C' THEN 'ZC'
                WHEN brand = 'Contax' AND model = 'IIa' THEN 'CIIA'
                WHEN brand = 'Contax' AND model = 'G1' THEN 'CG1'
                WHEN brand = 'Mamiya' AND model = 'RB67 Pro SD' THEN 'M67'
                WHEN brand = 'Intrepid' AND model = '4x5 Black Edition' THEN 'I45'
                WHEN brand = 'Meopta' AND model = 'Flexaret III' THEN 'FLEX'
                WHEN brand = 'Yashica' AND model = 'Electro 35 GSN' THEN 'Y35'
                WHEN brand = 'Kodak' AND model = 'Chevron' THEN 'KCHV'
                WHEN brand = 'Kodak' AND model = 'No. 2 Brownie, Model D' THEN 'KBRN'
                WHEN brand = 'Rollei' AND model = 'XF 35' THEN 'RXF'
                WHEN brand = 'Nikon' AND model = '35Ti' THEN 'N35'
                WHEN brand = 'Minolta' AND model = 'Hi-Matic F' THEN 'MHIF'
                WHEN brand = 'Canon' AND model = 'Canonet G-III QL' THEN 'CQL17'
                WHEN brand = 'Zeiss Ikon' AND model = 'Contessamat SBE' THEN 'ZSBE'
                WHEN brand = 'Diana' AND model = 'F' THEN 'DI'
                WHEN brand = 'Voigtländer' AND model = 'VSL 1' THEN 'VSLQ'
                WHEN brand = 'Mamiya' AND model = 'DSX 1000' THEN 'MDSX'
                ELSE prefix
             END",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Clear the prefixes set on seeded cameras (best-effort reversal — a fresh
        // DB had none). The 2 new cameras are deleted wholesale further down.
        db.execute_unprepared(
            "UPDATE cameras SET prefix = NULL WHERE brand || '|' || model IN (
                'Nikon|FE', 'Nikon|F2SB Photomic', 'Nikon|N80', 'Nikon|Nikkormat FTn',
                'Minolta|SR-T 101', 'Minolta|XD-11', 'Minolta|XD-7', 'Minolta|XE-1',
                'Olympus|OM-1n', 'Pentax|K1000', 'Asahi Pentax|SV', 'Pentax|Spotmatic SP',
                'Fujica|ST705', 'Fujica|ST901', 'Voigtländer|VSL 1 (TM)',
                'Contax|AX', 'Contax|RTSIII', 'Contax|139 Quartz', 'Contax|S2',
                'Leica|R6', 'Leica|Leicaflex SL', 'Mamiya|ZM', 'Zorki|C',
                'Contax|IIa', 'Contax|G1', 'Mamiya|RB67 Pro SD', 'Intrepid|4x5 Black Edition',
                'Meopta|Flexaret III', 'Yashica|Electro 35 GSN', 'Kodak|Chevron',
                'Kodak|No. 2 Brownie, Model D', 'Rollei|XF 35', 'Nikon|35Ti',
                'Minolta|Hi-Matic F', 'Canon|Canonet G-III QL', 'Zeiss Ikon|Contessamat SBE',
                'Diana|F', 'Voigtländer|VSL 1', 'Mamiya|DSX 1000'
            )",
        )
        .await?;

        // Revert the Nikkormat correction.
        db.execute_unprepared(
            "UPDATE cameras SET model = 'Nikkormat FT'
             WHERE brand = 'Nikon' AND model = 'Nikkormat FTn'",
        )
        .await?;

        // Unwire the Intrepid default lens + junction (the lens itself is a
        // standalone Rodenstock, deleted with the other standalone lenses below).
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = NULL
             WHERE brand = 'Intrepid' AND model = '4x5 Black Edition'",
        )
        .await?;
        db.execute_unprepared(
            "DELETE FROM camera_lenses WHERE
                camera_id = (SELECT id FROM cameras WHERE brand = 'Intrepid' AND model = '4x5 Black Edition')
                AND lens_id = (SELECT id FROM lenses WHERE brand = 'Rodenstock' AND model = 'Sironar-N 240mm f/5.6 MC')",
        )
        .await?;

        // Tear down the 2 new fixed-lens cameras: null default, delete junction,
        // delete their lenses, delete the cameras.
        db.execute_unprepared(
            "UPDATE cameras SET default_lens_id = NULL WHERE brand || '|' || model IN (
                'Olympus|Stylus Zoom 105', 'Ansco|Junior Model 1A'
            )",
        )
        .await?;
        db.execute_unprepared(
            "DELETE FROM camera_lenses WHERE camera_id IN (
                SELECT id FROM cameras WHERE brand || '|' || model IN (
                    'Olympus|Stylus Zoom 105', 'Ansco|Junior Model 1A'
                )
            )",
        )
        .await?;
        db.execute_unprepared(
            "DELETE FROM lenses WHERE lens_mount_id = (SELECT id FROM lens_mounts WHERE name = 'Fixed Lens')
             AND brand || '|' || IFNULL(model, '') IN (
                'Olympus|Zoom 38-105mm', 'Ansco|Meniscus'
             )",
        )
        .await?;
        db.execute_unprepared(
            "DELETE FROM cameras WHERE brand || '|' || model IN (
                'Olympus|Stylus Zoom 105', 'Ansco|Junior Model 1A'
            )",
        )
        .await?;

        // Delete the 13 standalone import-added lenses.
        db.execute_unprepared(
            "DELETE FROM lenses WHERE brand || '|' || IFNULL(model, '') IN (
                'Mamiya|90mm K/L', 'Exakta|28mm f/2.8 Macro', 'Mamiya|Sekor E 28mm f/2.8',
                'Mamiya|645 Sekor C N 80mm f/2.8 (adapted)', 'Mamiya|Sekor E 135mm f/2.8',
                'Asahi|Super-Takumar 85mm f/1.9 (M42, adapted)',
                'Asahi|Super-Takumar 105mm f/2.8 (M42, adapted)',
                'Asahi|SMC Takumar 200mm f/4',
                'Tokina|AT-X 116 PRO DX 11-16mm f/2.8 (borrowed)',
                'Asahi|Super-Takumar 135mm f/3.5', 'Nikon|Nikkor 50mm f/1.8 AF-D',
                'Rodenstock|Sironar-N 240mm f/5.6 MC',
                'Tokina|Zoom that came on the Fuji ST-901'
            )",
        )
        .await?;

        // Delete the 33 import-added film stocks.
        db.execute_unprepared(
            "DELETE FROM film_stocks WHERE brand || '|' || name || '|' || format IN (
                'Agfa|APX 100|135', 'Agfa|APX 400|135', 'Agfa|Vista Plus 200|135',
                'ArsImago|320|120', 'Bergger|Pancro 400|135', 'CatLabs|X Film 80|4x5',
                'CineStill|50D|135', 'CineStill|800T|135', 'CineStill|BwXX|135',
                'FPP|RetroChrome 400|135', 'FPP|Tasma NK-II 100|135',
                'Fujifilm|Acros 100|135', 'Fujifilm|Fujicolor 200|135',
                'Fujifilm|Natura 1600|135', 'Fujifilm|Neopan Acros 100 II|135',
                'Fujifilm|Sensia 100|135', 'Fujifilm|Superia 100|135',
                'Fujifilm|Superia 1600|135', 'Fujifilm|Superia 200|135',
                'Fujifilm|Superia 400|135', 'Holga|400 BW|135', 'Ilford|SFX 200|135',
                'Kodak|Eastman Double-X 5222|135', 'Kodak|Gold 400|135',
                'Kodak|Gold 800|135', 'Kodak|Hawkeye Super Color 200-400|135',
                'Kodak|Hawkeye Surveillance Film|135', 'Kodak|Kodacolor II|120',
                'Kodak|Max 400|135', 'Kodak|Portra 160VC|120', 'Kodak|ProImage 100|135',
                'Quality Photo|Best Choice Brand 200|135', 'Rollei|CN 200|135'
            )",
        )
        .await?;

        Ok(())
    }
}

/// Guarded lens insert on the natural key (brand + model + mount). Re-run-safe:
/// `WHERE NOT EXISTS` makes a repeat (or a live row that predates this migration)
/// a no-op. `mount` MUST be the exact stored `lens_mounts.name`.
async fn insert_lens(
    db: &SchemaManagerConnection<'_>,
    brand: &str,
    model: &str,
    mount: &str,
    focal_length: Option<&str>,
    max_aperture: Option<&str>,
) -> Result<(), DbErr> {
    let (brand, model, mount) = (sql_str(brand), sql_str(model), sql_str(mount));
    let focal = sql_opt(focal_length);
    let aperture = sql_opt(max_aperture);
    db.execute_unprepared(&format!(
        "INSERT INTO lenses (brand, lens_mount_id, model, focal_length, max_aperture, created_at, updated_at)
         SELECT {brand}, (SELECT id FROM lens_mounts WHERE name = {mount}), {model}, {focal}, {aperture}, datetime('now'), datetime('now')
         WHERE NOT EXISTS (
             SELECT 1 FROM lenses
             WHERE brand = {brand} AND IFNULL(model, '') = {model}
               AND lens_mount_id = (SELECT id FROM lens_mounts WHERE name = {mount})
         )",
    ))
    .await?;
    Ok(())
}

/// Guarded camera insert on the natural key (brand + model). Re-run-safe. The
/// prefix is intentionally NOT set here — the CASE UPDATE in `up` sets prefixes
/// uniformly for every seeded camera, so it reaches a pre-existing live row too.
/// `mount` MUST be the exact stored `lens_mounts.name`.
async fn insert_camera(
    db: &SchemaManagerConnection<'_>,
    brand: &str,
    model: &str,
    format: &str,
    mount: &str,
    camera_type: &str,
) -> Result<(), DbErr> {
    let (brand, model, format, mount, camera_type) = (
        sql_str(brand),
        sql_str(model),
        sql_str(format),
        sql_str(mount),
        sql_str(camera_type),
    );
    db.execute_unprepared(&format!(
        "INSERT INTO cameras (brand, model, format, lens_mount_id, camera_type, created_at, updated_at)
         SELECT {brand}, {model}, {format}, (SELECT id FROM lens_mounts WHERE name = {mount}), {camera_type}, datetime('now'), datetime('now')
         WHERE NOT EXISTS (
             SELECT 1 FROM cameras WHERE brand = {brand} AND model = {model}
         )",
    ))
    .await?;
    Ok(())
}

/// Link an existing camera's `default_lens_id` to an existing lens and record the
/// junction row, both keyed by natural name. Junction insert is `INSERT OR IGNORE`
/// (composite PK) and the UPDATE is naturally idempotent.
async fn link_default_lens(
    db: &SchemaManagerConnection<'_>,
    camera_brand: &str,
    camera_model: &str,
    lens_brand: &str,
    lens_model: &str,
) -> Result<(), DbErr> {
    let (camera_brand, camera_model, lens_brand, lens_model) = (
        sql_str(camera_brand),
        sql_str(camera_model),
        sql_str(lens_brand),
        sql_str(lens_model),
    );
    db.execute_unprepared(&format!(
        "INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES (
            (SELECT id FROM cameras WHERE brand = {camera_brand} AND model = {camera_model}),
            (SELECT id FROM lenses WHERE brand = {lens_brand} AND IFNULL(model, '') = {lens_model})
        )",
    ))
    .await?;
    db.execute_unprepared(&format!(
        "UPDATE cameras SET default_lens_id = (SELECT id FROM lenses WHERE brand = {lens_brand} AND IFNULL(model, '') = {lens_model})
         WHERE brand = {camera_brand} AND model = {camera_model}",
    ))
    .await?;
    Ok(())
}

/// Render a string as an escaped, single-quoted SQL literal (doubling any embedded
/// quote). The seed values here are fixed literals with no apostrophes, but the
/// escape keeps these reusable helpers from emitting malformed SQL if a future
/// value ever contains one.
fn sql_str(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Render an optional string as an escaped SQL literal or `NULL`.
fn sql_opt(v: Option<&str>) -> String {
    v.map(sql_str).unwrap_or_else(|| "NULL".to_string())
}
