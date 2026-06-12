//! Pre-migration DB snapshots (kammerz-466): `db::init` must snapshot a
//! file-backed database before applying pending migrations to an existing
//! catalog, and must NOT snapshot fresh databases or up-to-date ones.

use migration::{Migrator, MigratorTrait};
use sea_orm::ConnectionTrait;

/// Fresh temp dir + DB path for an isolated file-backed SQLite database.
///
/// Also forces snapshots ON: they default OFF in debug builds (which tests
/// are), and folding the override into the fixture means a future test can't
/// forget it and pass vacuously. Every test sets the same value, so the
/// process-global env var cannot race.
fn temp_db(name: &str) -> (std::path::PathBuf, std::path::PathBuf, String) {
    std::env::set_var("KAMMERZ_MIGRATION_SNAPSHOTS", "1");
    let dir = std::env::temp_dir().join(format!(
        "kammerz-snapshot-test-{name}-{}",
        std::process::id()
    ));
    // Clean slate in case a previous run left files behind.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("kammerz.db");
    let url = format!("sqlite:{}?mode=rwc", db_path.display());
    (dir, db_path, url)
}

/// List `kammerz.db.pre-migrate-*` snapshot files in `dir`.
fn snapshots_in(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut snaps: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with("kammerz.db.pre-migrate-"))
        })
        .map(|e| e.path())
        .collect();
    snaps.sort();
    snaps
}

#[tokio::test]
async fn fresh_db_takes_no_snapshot() {
    let (dir, _db_path, url) = temp_db("fresh");

    let db = kammerz::db::init(&url).await.unwrap();
    db.close().await.unwrap();

    assert!(
        snapshots_in(&dir).is_empty(),
        "a brand-new DB has nothing to protect — no snapshot expected"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn up_to_date_db_takes_no_snapshot() {
    let (dir, _db_path, url) = temp_db("up-to-date");

    let db = kammerz::db::init(&url).await.unwrap();
    db.close().await.unwrap();
    // Restart with no pending migrations — the normal steady-state boot.
    let db = kammerz::db::init(&url).await.unwrap();
    db.close().await.unwrap();

    assert!(
        snapshots_in(&dir).is_empty(),
        "no pending migrations — no snapshot expected"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `SELECT COUNT(*) FROM seaql_migrations` against a SQLite file.
async fn applied_migration_count(db_file: &std::path::Path) -> i64 {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite:{}", db_file.display()))
        .await
        .unwrap();
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM seaql_migrations")
        .fetch_one(&pool)
        .await
        .unwrap();
    pool.close().await;
    count
}

#[tokio::test]
async fn pending_migration_on_existing_db_takes_snapshot() {
    let (dir, db_path, url) = temp_db("pending");

    // First boot: apply all migrations, then roll the last one back so the
    // next boot sees an existing catalog with a genuinely pending migration —
    // the "upgrade the binary, restart the service" self-hoster flow.
    let db = kammerz::db::init(&url).await.unwrap();
    // Project rule: migrations run with FK enforcement OFF (init() re-enabled
    // it on this connection) — a future last migration whose down() rebuilds a
    // table would otherwise cascade-delete child rows here. Note this test
    // also assumes the newest migration has a working down().
    db.execute_unprepared("PRAGMA foreign_keys=OFF")
        .await
        .unwrap();
    Migrator::down(&db, Some(1)).await.unwrap();
    db.execute_unprepared("PRAGMA foreign_keys=ON")
        .await
        .unwrap();
    db.close().await.unwrap();

    let db = kammerz::db::init(&url).await.unwrap();
    db.close().await.unwrap();

    let snaps = snapshots_in(&dir);
    assert_eq!(
        snaps.len(),
        1,
        "expected exactly one pre-migration snapshot"
    );

    // The snapshot must be a valid SQLite DB capturing PRE-migration state.
    // Assert generically (no hard-coded knowledge of the latest migration):
    // the snapshot's seaql_migrations count is exactly one less than the live
    // re-migrated DB's — holds for any future migration appended to the chain.
    let snap_count = applied_migration_count(&snaps[0]).await;
    let live_count = applied_migration_count(&db_path).await;
    assert_eq!(
        snap_count + 1,
        live_count,
        "snapshot must reflect the pre-migration state (one fewer applied migration)"
    );

    // A subsequent up-to-date boot must not add another snapshot.
    let db = kammerz::db::init(&url).await.unwrap();
    db.close().await.unwrap();
    assert_eq!(snapshots_in(&dir).len(), 1);

    std::fs::remove_dir_all(&dir).unwrap();
}
