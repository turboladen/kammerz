use std::net::SocketAddr;
use std::str::FromStr;

use axum::http::{header, StatusCode, Uri};
use axum::response::IntoResponse;
use rust_embed::Embed;
use sqlx::sqlite::SqliteConnectOptions;
use time::Duration as TimeDuration;
use tower_http::trace::TraceLayer;
use tower_sessions::{cookie::SameSite, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

use kammerz::config::AppConfig;
use kammerz::error::AppError;
use kammerz::{db, routes, AppState};

#[derive(Embed)]
#[folder = "frontend/build"]
struct Assets;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Generate the argon2 hash for KAMMERZ_PASSWORD_HASH. Reads the password from
    // stdin (never argv) to keep it out of shell history and `ps` output:
    //   interactive: `kammerz hash-password`            (prompts, echo off)
    //   piped:       `echo -n <pw> | kammerz hash-password`  or  `kammerz hash-password < secret.txt`
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("hash-password") {
        if args.get(2).is_some_and(|a| a != "-") {
            eprintln!("error: do not pass the password as an argument (it leaks into shell history / ps).");
            eprintln!("usage: kammerz hash-password            # prompts on a TTY");
            eprintln!("       echo -n <pw> | kammerz hash-password");
            std::process::exit(2);
        }
        use std::io::{IsTerminal, Read};
        let pw = if std::io::stdin().is_terminal() {
            rpassword::prompt_password("Password: ").expect("failed to read password")
        } else {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).expect("failed to read stdin");
            s.trim_end_matches(['\n', '\r']).to_string()
        };
        println!("{}", kammerz::auth::password::hash_password(&pw).unwrap());
        return;
    }

    tracing_subscriber::fmt::init();

    // Surface the build version first thing so any deployed binary (NAS or dev)
    // identifies itself in the log even if boot fails later. Also reported by
    // GET /api/health for remote checks.
    tracing::info!("kammerz v{} starting", env!("CARGO_PKG_VERSION"));

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| db::default_db_url());
    let db = db::init(&db_url).await.expect("database init failed");

    let config = AppConfig::from_env();
    match &config.password_hash {
        None => {
            tracing::warn!(
                "KAMMERZ_PASSWORD_HASH is not set — running in OPEN (no-auth) mode. \
                 Set it for any network-reachable deployment."
            );
        }
        Some(hash) if !kammerz::auth::password::is_valid_hash(hash) => {
            // Fail fast: a hash that doesn't parse means every login would fail
            // with "incorrect password" and zero diagnostics. The usual culprit is
            // dotenvy's $-substitution mangling an UNQUOTED argon2 hash in .env
            // ('$argon2id$v=19$...' → '=19=19456,t=2,p=1').
            eprintln!(
                "error: KAMMERZ_PASSWORD_HASH is not a valid argon2 hash — did $-substitution \
                 in your .env mangle it? Single-quote the value, e.g.:\n\
                 \tKAMMERZ_PASSWORD_HASH='$argon2id$v=19$...'\n\
                 Generate a fresh hash with: echo -n <pw> | kammerz hash-password"
            );
            std::process::exit(1);
        }
        Some(_) => {}
    }

    // Session store: a separate sqlx pool against the same SQLite file (path
    // resolved by the same helper as db::init so the two pools never diverge).
    // busy_timeout matches the data pool so a session write that collides with a
    // catalog write waits instead of failing fast with SQLITE_BUSY.
    let session_pool = sqlx::SqlitePool::connect_with(
        SqliteConnectOptions::from_str(db::sqlite_path(&db_url))
            .expect("invalid DATABASE_URL for session store")
            .create_if_missing(true)
            .busy_timeout(db::busy_timeout())
            .foreign_keys(true),
    )
    .await
    .expect("session store pool");
    let session_store = SqliteStore::new(session_pool.clone());
    session_store.migrate().await.expect("session store migrate");

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(config.secure_cookies)
        .with_same_site(SameSite::Lax)
        .with_http_only(true)
        .with_expiry(Expiry::OnInactivity(TimeDuration::days(30)));

    let state = AppState { db: db.clone(), config };

    let app = routes::create_router(state)
        .fallback(serve_spa)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3002);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind");
    tracing::info!("kammerz listening on http://0.0.0.0:{port}");
    // `into_make_service_with_connect_info` installs `ConnectInfo<SocketAddr>` on
    // each request — the login rate-limiter's `PeerIpKeyExtractor` reads it to key
    // throttling by client IP.
    //
    // `with_graceful_shutdown` makes SIGTERM/SIGINT (e.g. `systemctl restart
    // kammerz`) drain in-flight requests instead of hard-killing them mid-write.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server error");

    // Post-drain cleanup: checkpoint the WAL back into kammerz.db so a copy of
    // the main DB file taken while the service is stopped is complete (a live
    // WAL would otherwise hold un-checkpointed writes), then close both pools.
    use sea_orm::ConnectionTrait;
    if let Err(e) = db.execute_unprepared("PRAGMA wal_checkpoint(TRUNCATE)").await {
        tracing::warn!("WAL checkpoint on shutdown failed: {e}");
    }
    session_pool.close().await;
    if let Err(e) = db.close().await {
        tracing::warn!("closing database connection failed: {e}");
    }
    tracing::info!("shutdown complete");
}

/// Resolves when the process receives SIGINT (Ctrl-C) or, on unix, SIGTERM
/// (systemd's default stop signal). Passed to `with_graceful_shutdown` so axum
/// stops accepting new connections and drains in-flight requests first.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install SIGINT handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received — draining in-flight requests");
}

fn is_route_like(path: &str) -> bool {
    if path.is_empty() || path.starts_with("_app/") {
        return false;
    }
    let last = path.rsplit('/').next().unwrap_or(path);
    !last.contains('.')
}

async fn serve_spa(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") {
        // Reuse the shared error envelope so unmatched /api/* 404s are byte-identical
        // to every handler error (frontend reads `error.code` / `error.message`).
        return AppError::NotFound("not found".to_string()).into_response();
    }
    let (asset, mime_path) = if path.is_empty() {
        (Assets::get("index.html"), "index.html")
    } else {
        match Assets::get(path) {
            Some(f) => (Some(f), path),
            None if is_route_like(path) => (Assets::get("index.html"), "index.html"),
            None => (None, path),
        }
    };
    match asset {
        Some(content) => {
            let mime = mime_guess::from_path(mime_path).first_or_octet_stream().as_ref().to_string();
            let cache = if path.starts_with("_app/immutable/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache.to_string())], content.data)
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
