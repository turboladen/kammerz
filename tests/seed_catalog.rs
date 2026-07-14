//! Guards the catalog-sync seed migration (m..027) against silent drift.
//!
//! m..027 backfills the seed data with the rows the 2026-06 Apple Notes/Numbers
//! import added straight into the live DB (kammerz-1gi film, kammerz-94b lenses,
//! kammerz-lxg cameras/prefixes/Nikkormat). These tests run the REAL migration
//! set on a fresh in-memory DB (via `open_app_with_db`) and assert the migration
//! actually produced those rows + the structural wiring — mirroring how
//! `status_flows.rs` gates the status fixture. If a future edit drops a row or
//! breaks the fixed-lens/default wiring, `cargo test` fails.
//!
//! Canary counts are exact snapshots (100/74/50/17). A future seed migration that
//! legitimately adds catalog rows must bump them here — that is the intended
//! trip-wire, not a nuisance.

mod common;

use common::open_app_with_db;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

// ── Canary counts ───────────────────────────────────────────────────

#[tokio::test]
async fn seed_catalog_counts_match_live_snapshot() {
    let (_app, db) = open_app_with_db().await;

    assert_eq!(
        entity::film_stock::Entity::find().count(&db).await.unwrap(),
        100,
        "film_stocks count drifted — a seed row was added/removed without updating this canary"
    );
    assert_eq!(
        entity::lens::Entity::find().count(&db).await.unwrap(),
        74,
        "lenses count drifted"
    );
    assert_eq!(
        entity::camera::Entity::find().count(&db).await.unwrap(),
        50,
        "cameras count drifted"
    );
    assert_eq!(
        entity::camera_lens::Entity::find()
            .count(&db)
            .await
            .unwrap(),
        17,
        "camera_lenses junction count drifted"
    );
}

// ── kammerz-1gi: representative film stocks ──────────────────────────

#[tokio::test]
async fn seed_catalog_has_import_film_stocks() {
    let (_app, db) = open_app_with_db().await;

    for (brand, name) in [
        ("CineStill", "50D"),
        ("Fujifilm", "Superia 400"),
        ("CatLabs", "X Film 80"),
        ("Kodak", "Gold 400"),
        ("Rollei", "CN 200"),
    ] {
        let found = entity::film_stock::Entity::find()
            .filter(entity::film_stock::Column::Brand.eq(brand))
            .filter(entity::film_stock::Column::Name.eq(name))
            .one(&db)
            .await
            .unwrap();
        assert!(found.is_some(), "missing import film stock: {brand} {name}");
    }
}

// ── kammerz-94b: standalone lens + normalization ─────────────────────

#[tokio::test]
async fn seed_catalog_has_rodenstock_lens_on_compur3() {
    let (_app, db) = open_app_with_db().await;

    let mount = entity::lens_mount::Entity::find()
        .filter(entity::lens_mount::Column::Name.eq("Compur #3"))
        .one(&db)
        .await
        .unwrap()
        .expect("Compur #3 mount exists");

    let lens = entity::lens::Entity::find()
        .filter(entity::lens::Column::Brand.eq("Rodenstock"))
        .filter(entity::lens::Column::Model.eq("Sironar-N 240mm f/5.6 MC"))
        .one(&db)
        .await
        .unwrap()
        .expect("Rodenstock Sironar-N 240mm seeded");

    assert_eq!(
        lens.lens_mount_id, mount.id,
        "Rodenstock lens must be on the Compur #3 mount"
    );
}

/// The one lens the import stored with a display-breaking `f/` prefix / `mm`
/// suffix must be BARE — else it double-renders as `f/f/4.5-8.9` / `38-105mmmm`
/// (kammerz-jd1). This covers the fresh-DB SEED path (the values written by
/// `insert_lens`). The sibling repair UPDATE that fixes a pre-existing live row
/// with `f/4.5-8.9` can't run against this already-normalized fresh DB, so it is
/// verified out-of-band by applying m..027 to a copy of the live catalog at
/// deploy time (both end at bare values, no duplicate row).
#[tokio::test]
async fn seed_catalog_normalizes_olympus_zoom_lens() {
    let (_app, db) = open_app_with_db().await;

    let lens = entity::lens::Entity::find()
        .filter(entity::lens::Column::Brand.eq("Olympus"))
        .filter(entity::lens::Column::Model.eq("Zoom 38-105mm"))
        .one(&db)
        .await
        .unwrap()
        .expect("Olympus Zoom 38-105mm seeded");

    assert_eq!(
        lens.max_aperture.as_deref(),
        Some("4.5-8.9"),
        "Olympus zoom max_aperture must be bare (no f/ prefix)"
    );
    assert_eq!(
        lens.focal_length.as_deref(),
        Some("38-105"),
        "Olympus zoom focal_length must be bare (no mm suffix)"
    );
}

// ── kammerz-lxg: cameras, fixed-lens wiring, prefixes, correction ────

/// Both new fixed-lens cameras must have their default lens wired AND a matching
/// junction row (the fixed-lens structural invariant).
#[tokio::test]
async fn seed_catalog_fixed_lens_cameras_are_wired() {
    let (_app, db) = open_app_with_db().await;

    for (brand, model, prefix) in [
        ("Olympus", "Stylus Zoom 105", "OX"),
        ("Ansco", "Junior Model 1A", "AJ"),
    ] {
        let camera = entity::camera::Entity::find()
            .filter(entity::camera::Column::Brand.eq(brand))
            .filter(entity::camera::Column::Model.eq(model))
            .one(&db)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("camera {brand} {model} seeded"));

        assert_eq!(
            camera.prefix.as_deref(),
            Some(prefix),
            "{brand} {model} prefix"
        );

        let default_lens_id = camera
            .default_lens_id
            .unwrap_or_else(|| panic!("{brand} {model} must have a default lens wired"));

        let junction = entity::camera_lens::Entity::find()
            .filter(entity::camera_lens::Column::CameraId.eq(camera.id))
            .filter(entity::camera_lens::Column::LensId.eq(default_lens_id))
            .one(&db)
            .await
            .unwrap();
        assert!(
            junction.is_some(),
            "{brand} {model} must have a camera_lenses junction row for its default lens"
        );
    }
}

/// The already-seeded Intrepid must be wired to the import-added Rodenstock as
/// its default lens.
#[tokio::test]
async fn seed_catalog_intrepid_default_lens_is_rodenstock() {
    let (_app, db) = open_app_with_db().await;

    let intrepid = entity::camera::Entity::find()
        .filter(entity::camera::Column::Brand.eq("Intrepid"))
        .filter(entity::camera::Column::Model.eq("4x5 Black Edition"))
        .one(&db)
        .await
        .unwrap()
        .expect("Intrepid seeded");

    let default_lens_id = intrepid
        .default_lens_id
        .expect("Intrepid must have a default lens wired");

    let lens = entity::lens::Entity::find_by_id(default_lens_id)
        .one(&db)
        .await
        .unwrap()
        .expect("Intrepid default lens exists");

    assert_eq!(lens.brand, "Rodenstock");
    assert_eq!(lens.model.as_deref(), Some("Sironar-N 240mm f/5.6 MC"));
}

#[tokio::test]
async fn seed_catalog_applies_nikkormat_correction() {
    let (_app, db) = open_app_with_db().await;

    let ftn = entity::camera::Entity::find()
        .filter(entity::camera::Column::Brand.eq("Nikon"))
        .filter(entity::camera::Column::Model.eq("Nikkormat FTn"))
        .one(&db)
        .await
        .unwrap();
    assert!(ftn.is_some(), "Nikkormat FTn (corrected model) must exist");
    assert_eq!(
        ftn.unwrap().prefix.as_deref(),
        Some("NFTN"),
        "corrected Nikkormat carries the NFTN prefix"
    );

    let ft = entity::camera::Entity::find()
        .filter(entity::camera::Column::Brand.eq("Nikon"))
        .filter(entity::camera::Column::Model.eq("Nikkormat FT"))
        .one(&db)
        .await
        .unwrap();
    assert!(ft.is_none(), "old Nikkormat FT model must be gone");
}

#[tokio::test]
async fn seed_catalog_sets_camera_prefixes() {
    let (_app, db) = open_app_with_db().await;

    // A sampled seeded camera that had no prefix before m..027.
    let leica = entity::camera::Entity::find()
        .filter(entity::camera::Column::Brand.eq("Leica"))
        .filter(entity::camera::Column::Model.eq("R6"))
        .one(&db)
        .await
        .unwrap()
        .expect("Leica R6 seeded");
    assert_eq!(leica.prefix.as_deref(), Some("LR6"));
}
