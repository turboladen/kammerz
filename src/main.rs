use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration as StdDuration;

use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::IntoResponse;
use rust_embed::Embed;
use sqlx::sqlite::SqliteConnectOptions;
use time::Duration as TimeDuration;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_sessions::{
    Expiry, SessionManagerLayer, cookie::SameSite, session_store::ExpiredDeletion,
};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::Level;
use tracing_subscriber::EnvFilter;

use kammerz::config::AppConfig;
use kammerz::error::AppError;
use kammerz::{AppState, db, routes};

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
            eprintln!(
                "error: do not pass the password as an argument (it leaks into shell history / ps)."
            );
            eprintln!("usage: kammerz hash-password            # prompts on a TTY");
            eprintln!("       echo -n <pw> | kammerz hash-password");
            std::process::exit(2);
        }
        use std::io::{IsTerminal, Read};
        let pw = if std::io::stdin().is_terminal() {
            rpassword::prompt_password("Password: ").expect("failed to read password")
        } else {
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .expect("failed to read stdin");
            s.trim_end_matches(['\n', '\r']).to_string()
        };
        println!("{}", kammerz::auth::password::hash_password(&pw).unwrap());
        return;
    }

    // Default to INFO so failed-login warnings and per-request access logs appear
    // out of the box. `fmt::init()` would use `EnvFilter::from_default_env()`, which
    // falls back to ERROR when RUST_LOG is unset (the shipped default — .env.example
    // leaves it commented), silencing the entire operability log. RUST_LOG still
    // overrides this when set (e.g. `tower_http=debug` for full request spans).
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Surface the build version first thing so any deployed binary (NAS or dev)
    // identifies itself in the log even if boot fails later. Also reported by
    // GET /api/health for remote checks.
    tracing::info!(
        "kammerz v{} ({}) starting",
        env!("CARGO_PKG_VERSION"),
        env!("KAMMERZ_BUILD_SHA")
    );

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| db::default_db_url());
    let db = db::init(&db_url).await.expect("database init failed");

    let config = AppConfig::from_env().unwrap_or_else(|e| {
        // A malformed config value (e.g. BIND_ADDR) is unrecoverable — exit
        // rather than start with a silently-wrong listener. Mirrors the
        // bad-hash fail-fast below so all startup config errors exit the same way.
        eprintln!("error: {e}");
        std::process::exit(1);
    });
    match &config.password_hash {
        None if config.bind_addr.is_loopback() => {
            // Open mode, but the kammerz-vlyu.13 default (or an explicit loopback
            // BIND_ADDR) keeps us off-host — the safe open-mode posture.
            tracing::warn!(
                "KAMMERZ_PASSWORD_HASH is not set — running in OPEN (no-auth) mode, bound to \
                 loopback ({}) only. Set KAMMERZ_PASSWORD_HASH before exposing this instance \
                 off-host via BIND_ADDR.",
                config.bind_addr
            );
        }
        None => {
            // Open mode AND reachable off-host: name the concrete exposure so an
            // operator who set a non-loopback BIND_ADDR without a hash sees exactly
            // what they've opened up (kammerz-vlyu.13).
            tracing::warn!(
                "KAMMERZ_PASSWORD_HASH is not set AND BIND_ADDR={} is non-loopback — the entire \
                 catalog, the DB backup download, and settings writes (including a billable \
                 claude_api_key) are reachable UNAUTHENTICATED by every host on the LAN/VPN. \
                 Set KAMMERZ_PASSWORD_HASH for any network-reachable deployment.",
                config.bind_addr
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

    // Announce which IP source the login rate limiter keys on — mirrors the
    // open-auth warning above so an operator can confirm trust-proxy mode took
    // effect. In trust-proxy mode the limiter reads X-Forwarded-For; without a
    // proxy that overwrites it, XFF is client-spoofable, hence the opt-in.
    if config.trust_proxy {
        tracing::info!(
            "KAMMERZ_TRUST_PROXY=true — login rate limiter keys on X-Forwarded-For. \
             Only safe behind a reverse proxy that overwrites that header."
        );
    } else {
        tracing::info!(
            "login rate limiter keys on the peer socket IP (set KAMMERZ_TRUST_PROXY=true when behind a trusted reverse proxy)"
        );
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
    session_store
        .migrate()
        .await
        .expect("session store migrate");

    // The SqliteStore filters expired sessions on load, but never deletes their
    // rows — so expired session records (stale auth artifacts) accumulate in the
    // SQLite file the operator backs up and carries around. Spawn tower-sessions'
    // recurring cleanup to purge them hourly. Clone the store first since the
    // layer takes ownership below. The loop exits on the first deletion error
    // (e.g. the DB file vanishing); we don't join the handle, so a stranded task
    // just stops purging — expired rows are still filtered out on load, never
    // served. (kammerz-135)
    tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(StdDuration::from_secs(60 * 60)),
    );

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(config.secure_cookies)
        .with_same_site(SameSite::Lax)
        .with_http_only(true)
        // 14-day inactivity window (kammerz-vlyu.25): a shared-password cookie
        // shouldn't stay valid for a full month if stolen/persisted, but this is
        // still comfortable for field use where days pass between sessions.
        .with_expiry(Expiry::OnInactivity(TimeDuration::days(14)));

    // Capture the bind address before `config` moves into AppState below.
    let bind_addr = config.bind_addr;

    let state = AppState {
        db: db.clone(),
        config,
        db_url: db_url.clone(),
    };

    // gzip/brotli negotiation on the way out (the primary remote path is a phone
    // over the field VPN; the ~568KB first-load bundle and GET /api/rolls JSON
    // compress 5-10x). The predicate and woff2 exclusion live in
    // `kammerz::compression` so this layer and the integration tests share one
    // definition.
    //
    // Layered last so it sits *outside* TraceLayer: trace then observes the
    // pre-compression response, keeping logged body sizes meaningful. It wraps
    // both create_router's /api routes and the serve_spa fallback because every
    // layer here applies after `.fallback(...)`.
    let app = routes::create_router(state)
        .fallback(serve_spa)
        .layer(session_layer)
        // Per-request access log at INFO. Both halves must be raised to INFO: the
        // span (carrying method + uri) and `on_response` (carrying status + latency)
        // each default to DEBUG, which the INFO-default subscriber drops — so a
        // status-only line, or no line at all, would leave a 404/422 untraceable.
        // Raising both makes each request log `request{method=… uri=…}: … status=…
        // latency=…`. Set RUST_LOG (e.g. `tower_http=debug`) for the full spans.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(kammerz::compression::compression_layer());

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3002);
    // Bind to the configured interface (BIND_ADDR, default 0.0.0.0). Loopback
    // (127.0.0.1) keeps the catalog off-host — the recommended posture behind a
    // reverse proxy or when running in OPEN (no-password) mode.
    let addr = SocketAddr::new(bind_addr, port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    tracing::info!("kammerz listening on http://{addr}");
    // `into_make_service_with_connect_info` installs `ConnectInfo<SocketAddr>` on
    // each request — the login rate-limiter's `PeerIpKeyExtractor` reads it to key
    // throttling by client IP (and `SmartIpKeyExtractor`, in trust-proxy mode,
    // falls back to it when no forwarding header is present).
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
    if let Err(e) = db
        .execute_unprepared("PRAGMA wal_checkpoint(TRUNCATE)")
        .await
    {
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
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install SIGINT handler");
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

async fn serve_spa(headers: HeaderMap, uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") {
        // Reuse the shared error envelope so unmatched /api/* 404s are byte-identical
        // to every handler error (frontend reads `error.code` / `error.message`).
        return AppError::NotFound("not found".to_string()).into_response();
    }
    // `cache_path` is the URL path (drives the immutable-vs-revalidate
    // Cache-Control choice); `mime_path` is the file actually served (an unknown
    // route falls back to index.html, so its content type is text/html). They
    // differ only on the SPA-fallback branch.
    let (asset, mime_path, cache_path) = if path.is_empty() {
        (Assets::get("index.html"), "index.html", "index.html")
    } else {
        match Assets::get(path) {
            Some(f) => (Some(f), path, path),
            None if is_route_like(path) => (Assets::get("index.html"), "index.html", path),
            None => (None, path, path),
        }
    };
    match asset {
        Some(content) => {
            let mime = mime_guess::from_path(mime_path)
                .first_or_octet_stream()
                .as_ref()
                .to_string();
            // Attach a strong, sha256-derived ETag and honor If-None-Match (304).
            // rust-embed precomputes the hash per embedded file. The SPA-fallback
            // path lands here too, so unknown routes revalidate index.html.
            kammerz::spa::asset_response(
                &headers,
                cache_path,
                content.data,
                &content.metadata.sha256_hash(),
                &mime,
            )
            .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
