//! Database connection. CRITICAL: migrations must run with foreign_keys=OFF
//! because table-rebuild migrations (CREATE new → INSERT → DROP old → RENAME)
//! cascade-delete child rows under FK enforcement (see migrations 019/020 and
//! CLAUDE.md). We then enable foreign_keys=ON for runtime queries.
//!
//! We use a SINGLE-connection pool (max=min=1) so the OFF→migrate→ON pragma
//! sequence is deterministic — every query runs on the same connection, and
//! the pragma toggles can't land on a different pooled connection. A single-user
//! catalog never needs concurrent writers, and SQLite serializes writes anyway.
//! This also keeps an in-memory test DB (`sqlite::memory:`) alive for the life
//! of the pool, so integration tests can migrate + query the same database.

use std::str::FromStr;
use std::time::Duration;

use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub fn busy_timeout() -> Duration {
    let ms = std::env::var("SQLITE_BUSY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5000);
    Duration::from_millis(ms)
}

/// Extract the bare SQLite file path from a `DATABASE_URL` — strips the
/// `sqlite:` scheme and any `?query`. Shared by `init()` and the session-store
/// setup in `main.rs` so the two pools always resolve the same file.
pub fn sqlite_path(db_url: &str) -> &str {
    let base = db_url.strip_prefix("sqlite:").unwrap_or(db_url);
    base.split('?').next().unwrap_or(base)
}

/// Connect (single persistent connection), migrate with FK OFF, enable FK ON.
pub async fn init(db_url: &str) -> Result<DatabaseConnection, DbErr> {
    // SqliteConnectOptions wants the path without the `sqlite:` scheme or `?query`.
    let base = sqlite_path(db_url);
    let opts = SqliteConnectOptions::from_str(base)
        .map_err(|e| DbErr::Custom(format!("bad sqlite url: {e}")))?
        .create_if_missing(true)
        .busy_timeout(busy_timeout());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .connect_with(opts)
        .await
        .map_err(|e| DbErr::Custom(format!("pool: {e}")))?;
    let db = sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);

    db.execute_unprepared("PRAGMA journal_mode=WAL").await?;
    snapshot_before_migrations(&db, base).await?;
    db.execute_unprepared("PRAGMA foreign_keys=OFF").await?; // critical during migrations
    Migrator::up(&db, None).await?;
    db.execute_unprepared("PRAGMA foreign_keys=ON").await?; // enforce at runtime
    // Prune only AFTER the migration succeeded. If a migration crashes midway
    // under a restart-on-failure supervisor, every restart sees it still
    // pending and takes a fresh snapshot of the now-corrupted DB — pruning
    // before `Migrator::up` would rotate out the one pristine pre-failure
    // snapshot after a few restarts.
    if !base.contains(":memory:") {
        prune_old_snapshots(base);
    }
    Ok(db)
}

/// How many pre-migration snapshots to keep next to the DB file.
const SNAPSHOTS_TO_KEEP: usize = 5;

/// Snapshot a file-backed DB before applying pending migrations.
///
/// Migrations run unconditionally at every startup, and `execute_unprepared()`
/// auto-commits per statement — a migration failing midway leaves partial data
/// persisted while the migration is NOT recorded in `seaql_migrations`, so it
/// re-runs on next start and can duplicate data (see CLAUDE.md "Migration raw
/// SQL gotcha"; migration 020 exists because a past migration destroyed data).
/// This turns a botched future migration from data loss into a file rename.
///
/// Skipped for in-memory DBs (tests) and for a fresh DB with no applied
/// migrations (nothing to lose). Uses `VACUUM INTO` via the open connection —
/// unlike a bare `fs::copy`, it produces a consistent snapshot even when a
/// prior crash left an un-checkpointed `-wal` file. A snapshot failure aborts
/// startup: migrating the only copy of the catalog without a safety net is
/// exactly what this guard exists to prevent.
async fn snapshot_before_migrations(db: &DatabaseConnection, path: &str) -> Result<(), DbErr> {
    if path.contains(":memory:") {
        return Ok(());
    }
    let pending = Migrator::get_pending_migrations(db).await?;
    if pending.is_empty() {
        return Ok(());
    }
    if Migrator::get_applied_migrations(db).await?.is_empty() {
        // Brand-new database — there is no catalog to protect yet.
        return Ok(());
    }
    let timestamp = chrono::Local::now().format("%Y%m%dT%H%M%S");
    let snapshot = format!("{path}.pre-migrate-{timestamp}");
    // VACUUM INTO fails if the target exists — which is what we want.
    db.execute_unprepared(&format!("VACUUM INTO '{}'", snapshot.replace('\'', "''")))
        .await?;
    tracing::info!(
        "{} pending migration(s) — snapshotted database to {snapshot}",
        pending.len()
    );
    Ok(())
}

/// Best-effort: keep only the newest `SNAPSHOTS_TO_KEEP` pre-migration
/// snapshots next to the DB file. The timestamp suffix sorts lexicographically,
/// so a plain sort orders snapshots oldest-first. Must only be called after
/// `Migrator::up` succeeds — see the comment in `init()`.
fn prune_old_snapshots(db_path: &str) {
    let path = std::path::Path::new(db_path);
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => std::path::Path::new("."),
    };
    let prefix = format!("{file_name}.pre-migrate-");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut snapshots: Vec<_> = entries
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with(&prefix))
        })
        .map(|e| e.path())
        .collect();
    if snapshots.len() <= SNAPSHOTS_TO_KEEP {
        return;
    }
    snapshots.sort();
    for old in &snapshots[..snapshots.len() - SNAPSHOTS_TO_KEEP] {
        if let Err(e) = std::fs::remove_file(old) {
            tracing::warn!("failed to prune old snapshot {}: {e}", old.display());
        } else {
            tracing::info!("pruned old pre-migration snapshot {}", old.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_keeps_newest_snapshots() {
        let dir = std::env::temp_dir().join(format!("kammerz-prune-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("kammerz.db");
        std::fs::write(&db_path, b"db").unwrap();
        for ts in [
            "20260101T000000",
            "20260102T000000",
            "20260103T000000",
            "20260104T000000",
            "20260105T000000",
            "20260106T000000",
            "20260107T000000",
        ] {
            std::fs::write(dir.join(format!("kammerz.db.pre-migrate-{ts}")), b"snap").unwrap();
        }

        prune_old_snapshots(db_path.to_str().unwrap());

        let mut remaining: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter(|n| n.starts_with("kammerz.db.pre-migrate-"))
            .collect();
        remaining.sort();
        assert_eq!(
            remaining,
            vec![
                "kammerz.db.pre-migrate-20260103T000000",
                "kammerz.db.pre-migrate-20260104T000000",
                "kammerz.db.pre-migrate-20260105T000000",
                "kammerz.db.pre-migrate-20260106T000000",
                "kammerz.db.pre-migrate-20260107T000000",
            ]
        );
        // The DB file itself is untouched.
        assert!(db_path.exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Default DB path. In dev: ./kammerz.db. Override with DATABASE_URL.
pub fn default_db_url() -> String {
    "sqlite:./kammerz.db?mode=rwc".to_string()
}
