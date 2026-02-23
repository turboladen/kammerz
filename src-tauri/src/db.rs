use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbErr};
use tauri::Manager;

pub async fn init(app_handle: &tauri::AppHandle) -> Result<DatabaseConnection, DbErr> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");

    std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");

    let db_path = app_data_dir.join("kammerz.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let db = Database::connect(&db_url).await?;

    // SQLite pragmas (performance — safe before migrations)
    db.execute_unprepared("PRAGMA journal_mode=WAL").await?;
    db.execute_unprepared("PRAGMA busy_timeout=5000").await?;

    // NOTE: foreign_keys=ON is set AFTER migrations, not before.
    // Migrations use table-rebuild patterns (CREATE new → INSERT → DROP old → RENAME)
    // and SQLite's DROP TABLE does an implicit DELETE when FK enforcement is on,
    // which triggers RESTRICT violations from referencing tables.

    // Handle migration from tauri-plugin-sql:
    // The old plugin tracked migrations in `_sqlx_migrations`. SeaORM uses
    // `seaql_migrations`. If the schema already exists (cameras table present),
    // we pre-populate SeaORM's tracking table so Migrator::up() skips the
    // initial schema and seed migrations.
    mark_existing_migrations(&db).await;

    migration::Migrator::up(&db, None).await?;

    // Enable FK enforcement for all runtime queries (after migrations are done)
    db.execute_unprepared("PRAGMA foreign_keys=ON").await?;

    Ok(db)
}

/// If the database already has the schema (from tauri-plugin-sql), mark our
/// SeaORM migrations as "applied" so they don't re-run.
async fn mark_existing_migrations(db: &DatabaseConnection) {
    // Check if the cameras table exists (proxy for "schema already set up").
    // We must use query_one() here — execute_unprepared() returns rows_affected,
    // which is always 0 for SELECT in SQLite.
    let schema_exists = db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='cameras'",
        ))
        .await
        .ok()
        .flatten()
        .is_some();

    if !schema_exists {
        // Fresh database — let Migrator::up() create everything
        return;
    }

    // Ensure seaql_migrations table exists
    let _ = db
        .execute_unprepared(
            "CREATE TABLE IF NOT EXISTS seaql_migrations (
                version VARCHAR NOT NULL PRIMARY KEY,
                applied_at BIGINT NOT NULL
            )",
        )
        .await;

    // Mark both migrations as applied (INSERT OR IGNORE so this is idempotent)
    let now = chrono::Utc::now().timestamp();
    let migrations = [
        "m20250101_000001_initial_schema",
        "m20250101_000002_seed_film_stocks",
    ];

    for migration in migrations {
        let sql = format!(
            "INSERT OR IGNORE INTO seaql_migrations (version, applied_at) VALUES ('{migration}', {now})"
        );
        if let Err(e) = db.execute_unprepared(&sql).await {
            log::warn!("Failed to mark migration {migration} as applied: {e}");
        }
    }

    log::info!("Existing schema detected — marked SeaORM migrations as applied");
}
