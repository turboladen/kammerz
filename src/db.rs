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
    db.execute_unprepared("PRAGMA foreign_keys=OFF").await?; // critical during migrations
    Migrator::up(&db, None).await?;
    db.execute_unprepared("PRAGMA foreign_keys=ON").await?; // enforce at runtime
    Ok(db)
}

/// Default DB path. In dev: ./kammerz.db. Override with DATABASE_URL.
pub fn default_db_url() -> String {
    "sqlite:./kammerz.db?mode=rwc".to_string()
}
