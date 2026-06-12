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
    // Single owner of the snapshot gate: build-profile/env decision plus the
    // in-memory exclusion, so the snapshot and prune paths can never diverge.
    let snapshots = snapshots_enabled() && !base.contains(":memory:");
    if snapshots {
        snapshot_before_migrations(&db, base).await?;
    }
    db.execute_unprepared("PRAGMA foreign_keys=OFF").await?; // critical during migrations
    Migrator::up(&db, None).await?;
    db.execute_unprepared("PRAGMA foreign_keys=ON").await?; // enforce at runtime

    // Prune only AFTER the migration succeeded. If a migration crashes midway
    // under a restart-on-failure supervisor, every restart sees it still
    // pending and takes a fresh snapshot of the now-corrupted DB — pruning
    // before `Migrator::up` would rotate out the one pristine pre-failure
    // snapshot after a few restarts.
    if snapshots {
        prune_old_snapshots(base);
    }
    Ok(db)
}

/// Run `VACUUM INTO` targeting `dest`. Fails if `dest` already exists.
///
/// The target path can't be a bind parameter for `VACUUM INTO`, so it is
/// embedded as a single-quoted SQL literal. Shared by the pre-migration
/// snapshot below and the `/api/backup` endpoint so the escaping and the
/// "target must not exist" semantics live in exactly one place.
pub async fn vacuum_into(db: &DatabaseConnection, dest: &str) -> Result<(), DbErr> {
    db.execute_unprepared(&format!("VACUUM INTO '{}'", dest.replace('\'', "''")))
        .await
        .map(|_| ())
}

/// Whether pre-migration snapshots are taken at startup.
///
/// Snapshots protect a deployed catalog across binary upgrades, so they
/// default to ON in release builds (the NAS deployment) and OFF in debug
/// builds — otherwise every local `cargo run` against a dev DB with pending
/// migrations would litter the working directory with snapshot files.
/// `KAMMERZ_MIGRATION_SNAPSHOTS` overrides in either direction; empty means
/// unset (matching `config.rs`), and an unrecognized value keeps the default
/// with a warning rather than silently disabling the safety net.
fn snapshots_enabled() -> bool {
    let default = !cfg!(debug_assertions);
    match std::env::var("KAMMERZ_MIGRATION_SNAPSHOTS") {
        Ok(v) => match v.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "on" | "yes" => true,
            "0" | "false" | "off" | "no" => false,
            "" => default,
            other => {
                tracing::warn!(
                    "unrecognized KAMMERZ_MIGRATION_SNAPSHOTS value {other:?} — \
                     keeping the default ({})",
                    if default { "on" } else { "off" }
                );
                default
            }
        },
        Err(_) => default,
    }
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
/// Only called when the gate in `init()` allows it (release builds / explicit
/// opt-in, file-backed DB). Additionally skipped when the DB has no user
/// tables (nothing to lose). Uses [`vacuum_into`] via the open connection —
/// unlike a bare `fs::copy`, it produces a consistent snapshot even when a
/// prior crash left an un-checkpointed `-wal` file. A snapshot failure aborts
/// startup: migrating the only copy of the catalog without a safety net is
/// exactly what this guard exists to prevent.
async fn snapshot_before_migrations(db: &DatabaseConnection, path: &str) -> Result<(), DbErr> {
    // "Nothing to protect" is decided by looking for user tables in
    // sqlite_master, NOT by `get_applied_migrations().is_empty()`: a populated
    // catalog whose seaql_migrations table is missing (restored from a partial
    // `sqlite3 .dump`, hand-repaired file) reports zero applied migrations and
    // is exactly the DB most in need of a snapshot — and the SeaORM migration
    // probes themselves `CREATE TABLE seaql_migrations` as a side effect, so
    // they must not be the first thing to touch a foreign file.
    if !has_user_tables(db).await? {
        return Ok(());
    }
    let pending = Migrator::get_pending_migrations(db).await?;
    if pending.is_empty() {
        return Ok(());
    }
    // Crash-loop guard: prune only runs after a successful migration, so a
    // persistently failing migration under a restart supervisor would add one
    // full-size snapshot per attempt forever. Once twice the keep-budget has
    // piled up, stop adding copies — the OLDEST snapshots (the pristine
    // pre-failure state) are the ones worth keeping.
    let existing = list_snapshots(path);
    if existing.len() >= SNAPSHOTS_TO_KEEP * 2 {
        tracing::warn!(
            "{} pre-migration snapshots already exist next to the DB — skipping a new one \
             (is a failing migration restart-looping?)",
            existing.len()
        );
        return Ok(());
    }
    // UTC so the names sort chronologically — prune relies on lexicographic
    // order, and local time is non-monotonic across DST/TZ changes.
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let snapshot = format!("{path}.pre-migrate-{timestamp}");
    // VACUUM INTO an adjacent temp file, then rename into place: an
    // interrupted VACUUM INTO can leave a corrupt partial output (SQLite docs
    // say to delete it), so the final name must only ever hold a complete
    // snapshot. The rename also makes a same-second re-run overwrite the
    // equivalent snapshot harmlessly instead of erroring on an existing target.
    let tmp = format!("{path}.pre-migrate.tmp");
    let _ = std::fs::remove_file(&tmp); // stale partial from a crashed attempt
    if let Err(e) = vacuum_into(db, &tmp).await {
        let _ = std::fs::remove_file(&tmp);
        return Err(DbErr::Custom(format!(
            "pre-migration snapshot failed ({tmp}): {e} — free disk space next to the DB, \
             or set KAMMERZ_MIGRATION_SNAPSHOTS=0 to skip snapshots"
        )));
    }
    std::fs::rename(&tmp, &snapshot).map_err(|e| {
        DbErr::Custom(format!(
            "failed to move pre-migration snapshot into place ({snapshot}): {e}"
        ))
    })?;
    tracing::info!(
        "{} pending migration(s) — snapshotted database to {snapshot}",
        pending.len()
    );
    Ok(())
}

/// Does the DB contain any user tables (ignoring SQLite internals and
/// `seaql_migrations`)? Decides whether there is catalog data worth
/// snapshotting before migrations run.
async fn has_user_tables(db: &DatabaseConnection) -> Result<bool, DbErr> {
    use sea_orm::{DbBackend, Statement};
    let row = db
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COUNT(*) AS n FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name <> 'seaql_migrations'",
        ))
        .await?;
    let n: i64 = match row {
        Some(r) => r.try_get("", "n")?,
        None => 0,
    };
    Ok(n > 0)
}

/// List `<db file>.pre-migrate-*` snapshots next to the DB file, sorted
/// oldest-first (the UTC timestamp suffix sorts lexicographically). Shared by
/// the prune pass and the crash-loop guard in `snapshot_before_migrations`.
fn list_snapshots(db_path: &str) -> Vec<std::path::PathBuf> {
    let path = std::path::Path::new(db_path);
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => std::path::Path::new("."),
    };
    let prefix = format!("{file_name}.pre-migrate-");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
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
    snapshots.sort();
    snapshots
}

/// Best-effort: keep only the newest `SNAPSHOTS_TO_KEEP` pre-migration
/// snapshots next to the DB file. Must only be called after `Migrator::up`
/// succeeds — see the comment in `init()`.
fn prune_old_snapshots(db_path: &str) {
    let snapshots = list_snapshots(db_path);
    if snapshots.len() <= SNAPSHOTS_TO_KEEP {
        return;
    }
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

    // Single test for all KAMMERZ_MIGRATION_SNAPSHOTS states: env vars are
    // process-global, so splitting these assertions into separate #[test]s
    // would race under the parallel test runner.
    #[test]
    fn snapshots_enabled_honors_build_profile_and_env_override() {
        // Asserted against cfg! rather than a literal so the test also holds
        // under `cargo test --release` (where the default flips to ON).
        let default = !cfg!(debug_assertions);
        std::env::remove_var("KAMMERZ_MIGRATION_SNAPSHOTS");
        assert_eq!(
            snapshots_enabled(),
            default,
            "unset env must fall back to the build-profile default"
        );
        std::env::set_var("KAMMERZ_MIGRATION_SNAPSHOTS", "1");
        assert!(
            snapshots_enabled(),
            "explicit opt-in must win over the default"
        );
        std::env::set_var("KAMMERZ_MIGRATION_SNAPSHOTS", "off");
        assert!(
            !snapshots_enabled(),
            "explicit opt-out must force snapshots off"
        );
        std::env::set_var("KAMMERZ_MIGRATION_SNAPSHOTS", "");
        assert_eq!(
            snapshots_enabled(),
            default,
            "empty value must mean unset, matching config.rs convention"
        );
        std::env::set_var("KAMMERZ_MIGRATION_SNAPSHOTS", "definitely-not-a-bool");
        assert_eq!(
            snapshots_enabled(),
            default,
            "unrecognized value must keep the default (with a warning), not silently disable"
        );
        std::env::remove_var("KAMMERZ_MIGRATION_SNAPSHOTS");
    }

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
