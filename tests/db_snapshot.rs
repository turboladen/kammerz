//! Pre-migration DB snapshots (kammerz-466): `db::init` must snapshot a
//! file-backed database before applying pending migrations to an existing
//! catalog, and must NOT snapshot fresh databases or up-to-date ones.

use migration::{Migrator, MigratorTrait};

/// Fresh temp dir + DB path for an isolated file-backed SQLite database.
fn temp_db(name: &str) -> (std::path::PathBuf, std::path::PathBuf, String) {
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

#[tokio::test]
async fn pending_migration_on_existing_db_takes_snapshot() {
    let (dir, _db_path, url) = temp_db("pending");

    // First boot: apply all migrations, then roll the last one back so the
    // next boot sees an existing catalog with a genuinely pending migration —
    // the "upgrade the binary, restart the service" self-hoster flow.
    let db = kammerz::db::init(&url).await.unwrap();
    Migrator::down(&db, Some(1)).await.unwrap();
    db.close().await.unwrap();

    let db = kammerz::db::init(&url).await.unwrap();
    db.close().await.unwrap();

    let snaps = snapshots_in(&dir);
    assert_eq!(snaps.len(), 1, "expected exactly one pre-migration snapshot");

    // The snapshot must be a valid SQLite DB capturing PRE-migration state:
    // migration 021 adds rolls.date_scanned, which the rolled-back snapshot
    // must not have, while the live (re-migrated) DB must.
    let snap_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite:{}", snaps[0].display()))
        .await
        .unwrap();
    let snap_cols: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM pragma_table_info('rolls')")
            .fetch_all(&snap_pool)
            .await
            .unwrap();
    assert!(
        !snap_cols.iter().any(|(n,)| n == "date_scanned"),
        "snapshot must reflect the pre-migration schema"
    );
    snap_pool.close().await;

    // A subsequent up-to-date boot must not add another snapshot.
    let db = kammerz::db::init(&url).await.unwrap();
    db.close().await.unwrap();
    assert_eq!(snapshots_in(&dir).len(), 1);

    std::fs::remove_dir_all(&dir).unwrap();
}
