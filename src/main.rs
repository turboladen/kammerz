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
use kammerz::{db, routes, AppState};

#[derive(Embed)]
#[folder = "frontend/build"]
struct Assets;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("hash-password") {
        let pw = args.get(2).expect("usage: kammerz hash-password <password>");
        println!("{}", kammerz::auth::password::hash_password(pw).unwrap());
        return;
    }

    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| db::default_db_url());
    let db = db::init(&db_url).await.expect("database init failed");

    let config = AppConfig::from_env();
    if config.password_hash.is_none() {
        tracing::warn!(
            "KAMMERZ_PASSWORD_HASH is not set — running in OPEN (no-auth) mode. \
             Set it for any network-reachable deployment."
        );
    }

    // Session store: a separate sqlx pool against the same SQLite file. The
    // store reads its own connection options (no migration FK toggling here).
    let session_base = db_url
        .strip_prefix("sqlite:")
        .unwrap_or(&db_url)
        .split('?')
        .next()
        .unwrap()
        .to_string();
    let session_pool = sqlx::SqlitePool::connect_with(
        SqliteConnectOptions::from_str(&session_base)
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true),
    )
    .await
    .expect("session store pool");
    let session_store = SqliteStore::new(session_pool);
    session_store.migrate().await.expect("session store migrate");

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(config.secure_cookies)
        .with_same_site(SameSite::Lax)
        .with_http_only(true)
        .with_expiry(Expiry::OnInactivity(TimeDuration::days(30)));

    let state = AppState { db, config: config.clone() };

    let app = routes::create_router(state)
        .fallback(serve_spa)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3001);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind");
    tracing::info!("kammerz listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("server error");
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
        return (StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            "{\"error\":{\"code\":\"NOT_FOUND\",\"message\":\"not found\"}}")
            .into_response();
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
