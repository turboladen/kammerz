//! Online SQLite backup. `GET /api/backup` runs `VACUUM INTO` against a temp
//! file and returns the snapshot as a download.
//!
//! The database runs in WAL mode (see `db.rs`), so a naive `cp kammerz.db`
//! while the server is running is unsafe: recent commits live in
//! `kammerz.db-wal`, and a copy taken mid-checkpoint can be stale or torn.
//! `VACUUM INTO` produces a consistent, WAL-free, single-file snapshot of the
//! live database — safe to take while the server is up. The README's
//! "Backups" section documents this endpoint plus the equivalent `sqlite3`
//! CLI invocation for shell-based backup jobs.

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use sea_orm::{ConnectionTrait, DatabaseConnection};

use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(download_backup))
}

/// Unique temp path for the snapshot. `VACUUM INTO` refuses to overwrite an
/// existing file, so the name embeds the pid and a nanosecond timestamp.
/// Under the hardened systemd unit, `PrivateTmp=true` keeps `/tmp` writable
/// (and private) despite `ProtectSystem=strict`.
fn temp_snapshot_path() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "kammerz-backup-{}-{nanos}.sqlite",
        std::process::id()
    ))
}

async fn download_backup(
    _: RequireAuth,
    State(db): State<DatabaseConnection>,
) -> AppResult<impl IntoResponse> {
    let path = temp_snapshot_path();
    let path_str = path
        .to_str()
        .ok_or_else(|| AppError::Internal("temp dir path is not valid UTF-8".to_string()))?;

    // The target path can't be a bind parameter for VACUUM INTO; embed it as a
    // single-quoted SQL literal (we generate the path, but escape defensively).
    let sql = format!("VACUUM INTO '{}'", path_str.replace('\'', "''"));
    let snapshot = match db.execute_unprepared(&sql).await {
        Ok(_) => tokio::fs::read(&path)
            .await
            .map_err(|e| AppError::Internal(format!("read backup snapshot: {e}"))),
        Err(e) => Err(AppError::Internal(format!("VACUUM INTO failed: {e}"))),
    };
    // Best-effort cleanup of the temp file on both success and failure.
    let _ = tokio::fs::remove_file(&path).await;
    let bytes = snapshot?;

    let filename = format!(
        "kammerz-backup-{}.db",
        chrono::Local::now().format("%Y-%m-%d")
    );
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/octet-stream".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    ))
}
