//! Re-run-safe seed helpers for the pre-m017 seed migrations (m013-016).
//!
//! `execute_unprepared` auto-commits each statement and a seed migration is only
//! recorded in `seaql_migrations` after its whole `up()` returns, so a crash
//! mid-migration re-runs from statement 1 on the next boot — silently duplicating
//! already-committed `cameras`/`lenses` rows (neither table has a unique key). These
//! helpers guard every insert on its natural key so a re-run is a no-op while a
//! clean first run stays byte-identical (every guard is true, so all inserts fire
//! in original order with the same AUTOINCREMENT ids).
//!
//! These migrations run BEFORE m017 renamed `lenses.name_on_lens` -> `model`, so the
//! lens helpers key on the historical `name_on_lens` column. m027's model-keyed
//! helpers are NOT reusable here for that reason. Junction inserts are handled
//! inline in m013 with `INSERT OR IGNORE` (composite PK), preserving that
//! migration's `MAX(id)` lens lookups verbatim (m020's MIN/MAX repair depends on
//! them). No UNIQUE index is added — the app deliberately supports duplicate gear.

use sea_orm_migration::prelude::*;

/// Guarded camera insert on the natural key (brand, model). Re-run-safe: a repeat
/// no-ops via `WHERE NOT EXISTS`. `mount` MUST be the exact stored `lens_mounts.name`.
pub(crate) async fn insert_camera(
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

/// Guarded lens insert on the pre-m017 natural key (brand, name_on_lens,
/// lens_mount_id). Re-run-safe via `WHERE NOT EXISTS`. `mount` MUST be the exact
/// stored `lens_mounts.name`.
pub(crate) async fn insert_lens(
    db: &SchemaManagerConnection<'_>,
    brand: &str,
    name_on_lens: &str,
    mount: &str,
    focal_length: Option<&str>,
    max_aperture: Option<&str>,
) -> Result<(), DbErr> {
    let (brand, name, mount) = (sql_str(brand), sql_str(name_on_lens), sql_str(mount));
    let focal = sql_opt(focal_length);
    let aperture = sql_opt(max_aperture);
    db.execute_unprepared(&format!(
        "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at)
         SELECT {brand}, (SELECT id FROM lens_mounts WHERE name = {mount}), {name}, {focal}, {aperture}, datetime('now'), datetime('now')
         WHERE NOT EXISTS (
             SELECT 1 FROM lenses
             WHERE brand = {brand} AND IFNULL(name_on_lens, '') = {name}
               AND lens_mount_id = (SELECT id FROM lens_mounts WHERE name = {mount})
         )",
    ))
    .await?;
    Ok(())
}

/// Ordinal-guarded lens insert for the ONLY intentional duplicate natural key in
/// the seed data: two `('Zeiss Ikon','Tessar 50mm f/2.8','Fixed Lens')` rows
/// (S310 + Contessamat), later disambiguated by id and relied on by m020's
/// MIN(id)/MAX(id) repair. A blanket `WHERE NOT EXISTS` would collapse them to one
/// and break m020. This guard keeps exactly `max_count` copies: the S310 insert
/// passes `1`, the Contessamat insert passes `2`, so a clean run fires both (in
/// order → distinct ascending ids) while any re-run leaves exactly two.
pub(crate) async fn insert_lens_ordinal(
    db: &SchemaManagerConnection<'_>,
    brand: &str,
    name_on_lens: &str,
    mount: &str,
    focal_length: Option<&str>,
    max_aperture: Option<&str>,
    max_count: u32,
) -> Result<(), DbErr> {
    let (brand, name, mount) = (sql_str(brand), sql_str(name_on_lens), sql_str(mount));
    let focal = sql_opt(focal_length);
    let aperture = sql_opt(max_aperture);
    db.execute_unprepared(&format!(
        "INSERT INTO lenses (brand, lens_mount_id, name_on_lens, focal_length, max_aperture, created_at, updated_at)
         SELECT {brand}, (SELECT id FROM lens_mounts WHERE name = {mount}), {name}, {focal}, {aperture}, datetime('now'), datetime('now')
         WHERE (
             SELECT COUNT(*) FROM lenses
             WHERE brand = {brand} AND IFNULL(name_on_lens, '') = {name}
               AND lens_mount_id = (SELECT id FROM lens_mounts WHERE name = {mount})
         ) < {max_count}",
    ))
    .await?;
    Ok(())
}

/// Render a string as an escaped, single-quoted SQL literal (doubling any embedded
/// quote). Seed values here are fixed literals, but the escape keeps these helpers
/// from emitting malformed SQL if a value ever contains an apostrophe.
fn sql_str(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Render an optional string as an escaped SQL literal or `NULL`.
fn sql_opt(v: Option<&str>) -> String {
    v.map(sql_str).unwrap_or_else(|| "NULL".to_string())
}

#[cfg(test)]
mod tests {
    use crate::Migrator;
    use sea_orm_migration::prelude::*;
    use sea_orm_migration::sea_orm::{
        ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement,
    };

    /// Fresh in-memory DB migrated through `n` migrations, FK enforcement OFF
    /// exactly as `db.rs` sets it before migrating. A single connection keeps the
    /// in-memory DB alive across statements.
    async fn migrated_to(n: u32) -> DatabaseConnection {
        let mut opt = ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1).min_connections(1);
        let db = Database::connect(opt).await.unwrap();
        db.execute_unprepared("PRAGMA foreign_keys=OFF")
            .await
            .unwrap();
        Migrator::up(&db, Some(n)).await.unwrap();
        db
    }

    /// Fresh in-memory DB with the FULL migration stack applied (as at server init).
    async fn migrated_full() -> DatabaseConnection {
        let mut opt = ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1).min_connections(1);
        let db = Database::connect(opt).await.unwrap();
        db.execute_unprepared("PRAGMA foreign_keys=OFF")
            .await
            .unwrap();
        Migrator::up(&db, None).await.unwrap();
        db
    }

    async fn scalar(db: &DatabaseConnection, sql: &str) -> i64 {
        db.query_one(Statement::from_string(
            db.get_database_backend(),
            sql.to_owned(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "v")
        .unwrap()
    }

    async fn count(db: &DatabaseConnection, table: &str) -> i64 {
        scalar(db, &format!("SELECT COUNT(*) AS v FROM {table}")).await
    }

    /// The two intentional Zeiss Tessar rows share this natural key (pre-m017
    /// `name_on_lens` column at Some(16)).
    async fn zeiss_tessar_count(db: &DatabaseConnection) -> i64 {
        scalar(
            db,
            "SELECT COUNT(*) AS v FROM lenses \
             WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8'",
        )
        .await
    }

    async fn zeiss_junctions(db: &DatabaseConnection, model: &str) -> i64 {
        scalar(
            db,
            &format!(
                "SELECT COUNT(*) AS v FROM camera_lenses cl \
                 JOIN cameras c ON c.id = cl.camera_id \
                 WHERE c.brand = 'Zeiss Ikon' AND c.model = '{model}'"
            ),
        )
        .await
    }

    /// The lens_id S310's single junction row points at (asserted == MIN(Tessar id)).
    async fn s310_junction_lens(db: &DatabaseConnection) -> i64 {
        scalar(
            db,
            "SELECT cl.lens_id AS v FROM camera_lenses cl \
             JOIN cameras c ON c.id = cl.camera_id \
             WHERE c.brand = 'Zeiss Ikon' AND c.model = 'S310'",
        )
        .await
    }

    async fn s310_default_lens(db: &DatabaseConnection) -> i64 {
        scalar(
            db,
            "SELECT default_lens_id AS v FROM cameras \
             WHERE brand = 'Zeiss Ikon' AND model = 'S310'",
        )
        .await
    }

    /// Re-running each seed migration's `up()` (the real crash-resume path — a
    /// migration re-runs from statement 1 until it records in `seaql_migrations`)
    /// must not duplicate any rows, and must keep the intentional Zeiss Tessar pair
    /// at exactly two with one junction per Zeiss camera. Stops at m016 so the
    /// schema still has the pre-m017 `name_on_lens` column the seeds reference.
    #[tokio::test]
    async fn seed_migrations_idempotent() {
        let db = migrated_to(16).await;

        // Absolute byte-identical anchors for the seed phase (counts through m016).
        assert_eq!(count(&db, "cameras").await, 48);
        assert_eq!(count(&db, "lenses").await, 59);
        assert_eq!(count(&db, "camera_lenses").await, 14);
        assert_eq!(zeiss_tessar_count(&db).await, 2);

        // Re-run every seed up() a second full time (superset of any partial crash).
        let manager = SchemaManager::new(&db);
        crate::m20260221_000013_seed_user_cameras::Migration
            .up(&manager)
            .await
            .unwrap();
        crate::m20260222_000014_seed_user_lenses::Migration
            .up(&manager)
            .await
            .unwrap();
        crate::m20260222_000015_seed_qbm_and_extras::Migration
            .up(&manager)
            .await
            .unwrap();
        crate::m20260222_000016_seed_user_gear_batch2::Migration
            .up(&manager)
            .await
            .unwrap();

        assert_eq!(
            count(&db, "cameras").await,
            48,
            "camera duplicated on re-run"
        );
        assert_eq!(count(&db, "lenses").await, 59, "lens duplicated on re-run");
        assert_eq!(
            count(&db, "camera_lenses").await,
            14,
            "junction duplicated on re-run"
        );
        assert_eq!(
            zeiss_tessar_count(&db).await,
            2,
            "Zeiss Tessar pair must stay exactly two"
        );

        // Pin the S310 -> lower-id-Tessar mapping so a regression back to MAX(id)
        // fails LOUDLY. With MAX(id), a re-run (both Tessars now present) would
        // resolve S310 to the Contessamat's higher-id lens: a spurious second S310
        // junction row (INSERT OR IGNORE won't dedup a different pair) plus a wrong
        // default that neither m019 (FK-off, no cascade) nor m020 (heals only NULLs)
        // repairs. MIN(id) is the correct, re-run-safe target.
        let tessar_min = scalar(
            &db,
            "SELECT MIN(id) AS v FROM lenses \
             WHERE brand = 'Zeiss Ikon' AND name_on_lens = 'Tessar 50mm f/2.8'",
        )
        .await;
        assert_eq!(
            zeiss_junctions(&db, "S310").await,
            1,
            "S310 must keep exactly one junction row after re-run"
        );
        assert_eq!(
            zeiss_junctions(&db, "Contessamat SBE").await,
            1,
            "Contessamat must keep exactly one junction row after re-run"
        );
        assert_eq!(
            s310_junction_lens(&db).await,
            tessar_min,
            "S310's junction must point at the lower-id (MIN) Tessar, not the Contessamat's"
        );
        assert_eq!(
            s310_default_lens(&db).await,
            tessar_min,
            "S310.default_lens_id must be the lower-id (MIN) Tessar, not the Contessamat's"
        );
    }

    /// The full migration stack yields the expected catalog size (byte-identical
    /// fresh install) and m020's MIN/MAX repair resolves the Zeiss Tessar pair to
    /// distinct lenses — S310 -> the lower id, Contessamat -> the higher.
    #[tokio::test]
    async fn full_migration_byte_identical_and_zeiss_invariant() {
        let db = migrated_full().await;

        assert_eq!(count(&db, "cameras").await, 50);
        assert_eq!(count(&db, "lenses").await, 74);
        assert_eq!(count(&db, "camera_lenses").await, 17);

        // Post-m017 the column is `model`.
        let min_id = scalar(
            &db,
            "SELECT MIN(id) AS v FROM lenses WHERE brand = 'Zeiss Ikon' AND model = 'Tessar 50mm f/2.8'",
        )
        .await;
        let max_id = scalar(
            &db,
            "SELECT MAX(id) AS v FROM lenses WHERE brand = 'Zeiss Ikon' AND model = 'Tessar 50mm f/2.8'",
        )
        .await;
        assert!(min_id < max_id, "the two Tessars must have distinct ids");

        let s310 = scalar(
            &db,
            "SELECT default_lens_id AS v FROM cameras WHERE brand = 'Zeiss Ikon' AND model = 'S310'",
        )
        .await;
        let contessamat = scalar(
            &db,
            "SELECT default_lens_id AS v FROM cameras WHERE brand = 'Zeiss Ikon' AND model = 'Contessamat SBE'",
        )
        .await;
        assert_eq!(s310, min_id, "S310 must default to the lower-id Tessar");
        assert_eq!(
            contessamat, max_id,
            "Contessamat must default to the higher-id Tessar"
        );
    }
}
