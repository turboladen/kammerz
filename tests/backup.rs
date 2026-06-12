//! Integration tests for the WAL-safe online backup endpoint (GET /api/backup).
//!
//! The snapshot test uses a file-backed temp DB instead of `sqlite::memory:`:
//! through sqlx, `VACUUM INTO` on an in-memory database silently produces no
//! output file. A real deployment is always file-backed (and in WAL mode),
//! which is exactly the scenario this endpoint exists for.

use axum::http::StatusCode;
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;
use common::{app_with_password, get, open_app_with_url};

#[tokio::test]
async fn backup_returns_sqlite_snapshot_as_download() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_path = std::env::temp_dir().join(format!(
        "kammerz-test-backup-{}-{nanos}.db",
        std::process::id()
    ));
    let app = open_app_with_url(&format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap())).await;

    let res = app.oneshot(get("/api/backup")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let disposition = res
        .headers()
        .get("content-disposition")
        .expect("content-disposition header")
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        disposition.starts_with("attachment; filename=\"kammerz-backup-"),
        "unexpected content-disposition: {disposition}"
    );

    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    // Every SQLite database file starts with this 16-byte magic header — a
    // torn/empty/non-DB response would fail here.
    assert!(
        bytes.len() > 16,
        "snapshot too small: {} bytes",
        bytes.len()
    );
    assert_eq!(&bytes[..16], b"SQLite format 3\0");

    // Clean up the temp DB and its WAL sidecars.
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", db_path.display()));
    }
}

#[tokio::test]
async fn backup_requires_auth_when_password_set() {
    let app = app_with_password("pw").await;
    let res = app.oneshot(get("/api/backup")).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
