use axum::extract::State;
use axum::routing::{get, post, MethodRouter};
use axum::{Json, Router};
use sea_orm::{DatabaseConnection, DbErr, TransactionError};
use serde_json::{json, Value};
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::{KeyExtractor, PeerIpKeyExtractor, SmartIpKeyExtractor};
use tower_governor::GovernorLayer;

use crate::auth::{handlers, rate_limit};
use crate::error::{AppError, AppResult};
use crate::AppState;

pub mod backup;
pub mod cameras;
pub mod development;
pub mod film_stocks;
pub mod import;
pub mod labs;
pub mod lens_mounts;
pub mod lenses;
pub mod rolls;
pub mod search;
pub mod settings;
pub mod shots;
pub mod stats;

pub fn create_router(state: AppState) -> Router {
    // Per-IP brute-force guard, scoped to the login route only (logout/me/business
    // routes are untouched). The key-extractor choice depends on whether we trust a
    // reverse proxy's `X-Forwarded-For`; both branches erase to the same
    // `MethodRouter<AppState>`. See `auth::rate_limit` for the rationale/constants.
    let login_route = if state.config.trust_proxy {
        login_route(SmartIpKeyExtractor)
    } else {
        login_route(PeerIpKeyExtractor)
    };

    Router::<AppState>::new()
        .route("/api/health", get(health))
        .route("/api/auth/login", login_route)
        .route("/api/auth/logout", post(handlers::logout))
        .route("/api/auth/me", get(handlers::me))
        .nest("/api/cameras", cameras::router())
        .nest("/api/maintenance", cameras::maintenance_router())
        .nest("/api/lenses", lenses::router())
        .nest("/api/lens-mounts", lens_mounts::router())
        .nest("/api/film-stocks", film_stocks::router())
        .nest("/api/labs", labs::router())
        .nest("/api/rolls", rolls::router())
        .nest("/api/shots", shots::router())
        .nest("/api/development", development::router())
        .nest("/api/search", search::router())
        .nest("/api/stats", stats::router())
        .nest("/api/settings", settings::router())
        .nest("/api/import", import::router())
        .nest("/api/backup", backup::router())
        .with_state(state)
}

/// Build the `POST /api/auth/login` route with the brute-force rate limiter
/// attached, keyed by `extractor`. Generic over the key extractor so
/// `create_router` can pick `PeerIpKeyExtractor` (the default — peer socket IP)
/// or `SmartIpKeyExtractor` (X-Forwarded-For aware, for trust-proxy mode) without
/// duplicating the route/limiter setup; the resulting `MethodRouter<AppState>`
/// type is identical either way. See `auth::rate_limit` for the tuning constants
/// and `on_governor_error` for the error envelope.
fn login_route<K>(extractor: K) -> MethodRouter<AppState>
where
    // axum's `MethodRouter::layer` requires the layered service to be
    // `Send + Sync + 'static`; the governor layer holds the extractor and an
    // `Arc<RateLimiter<K::Key, ...>>`, so both `K` and its key type inherit those
    // bounds. Both concrete extractors we use (Peer/Smart) and their `IpAddr` key
    // satisfy them.
    K: KeyExtractor + Send + Sync + 'static,
    K::Key: Send + Sync,
{
    let limiter = GovernorLayer::new(
        GovernorConfigBuilder::default()
            .burst_size(rate_limit::LOGIN_BURST_SIZE)
            .per_second(rate_limit::LOGIN_REPLENISH_SECONDS)
            .key_extractor(extractor)
            .finish()
            .expect("login rate-limit config is valid"),
    )
    .error_handler(rate_limit::on_governor_error);

    post(handlers::login).layer(limiter)
}

async fn health(State(db): State<DatabaseConnection>) -> AppResult<Json<Value>> {
    // Liveness probe used by systemd/uptime monitors, CI readiness, and
    // `just deploy`. Accepting the TCP connection isn't enough: the single
    // max=min=1 pool (src/db.rs) can be wedged by a stuck writer, and the DB
    // file can vanish if the NAS data dir is unmounted — both leave the process
    // up but unable to serve. Ping the connection so a dead DB reports 503 (the
    // CI/deploy probes use `curl -f`, so a non-2xx correctly fails readiness)
    // instead of a misleading healthy 200.
    db.ping()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("database ping failed: {e}")))?;

    // `version` identifies which build a deployment is running (the binary is
    // installed on a remote NAS, so the log line alone isn't always reachable);
    // `build` is the SHA `just deploy` greps for to confirm the new binary is
    // the one answering — keep both fields and their names stable.
    Ok(Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
        "build": env!("KAMMERZ_BUILD_SHA"),
    })))
}

/// The operation that produced a DB error, used to word a FOREIGN KEY violation
/// correctly. The same constraint ("FOREIGN KEY constraint failed") fires both
/// when a create/update references a missing/stale id and when a delete is
/// blocked by referencing rows — but those need opposite messages (kammerz-956).
#[derive(Clone, Copy)]
pub enum Op {
    /// An insert or update — a FK violation means the referenced row is gone.
    Write,
    /// A delete — a FK violation means other records still reference this one.
    Delete,
}

/// Map a DB error to a user-friendly message. Recognizes common SQLite constraint
/// errors and produces actionable text; falls back to the raw error otherwise.
/// Accepts DbErr, TransactionError<DbErr>, or any Display type.
///
/// This is the create/update/link helper — FK violations are worded as a missing
/// reference. Delete handlers route through [`friendly_delete_err`] /
/// [`friendly_txn_err`] with [`Op::Delete`] for the "still referenced" wording.
///
/// The `context` should be a noun phrase (e.g. "roll", "camera", "film stock").
pub fn friendly_err(context: &str, e: impl std::fmt::Display) -> String {
    friendly_err_op(context, Op::Write, e)
}

/// Op-aware core of [`friendly_err`]. Only the FOREIGN KEY branch varies by op;
/// every other message is identical regardless of operation.
fn friendly_err_op(context: &str, op: Op, e: impl std::fmt::Display) -> String {
    let raw = e.to_string();

    // SeaORM wraps SQLite errors (e.g. "Execution Error: ... UNIQUE constraint
    // failed: table.col"), so we search with `contains()` + extract the tail.

    // UNIQUE constraint failed: table.column
    if let Some(pos) = raw.find("UNIQUE constraint failed: ") {
        let rest = &raw[pos + "UNIQUE constraint failed: ".len()..];
        // Strip any trailing quote/paren that SeaORM's wrapping may add
        let rest = rest.trim_end_matches(|c: char| c == '"' || c == ')' || c.is_whitespace());
        let col = rest.split('.').last().unwrap_or("value").replace('_', " ");
        return format!("A {context} with that {col} already exists.");
    }
    if raw.contains("UNIQUE constraint failed") {
        return format!("A {context} with those values already exists.");
    }

    // FOREIGN KEY constraint failed — the message depends on the operation: a
    // create/update hit a missing reference; a delete is blocked by referrers.
    if raw.contains("FOREIGN KEY constraint failed") {
        return match op {
            Op::Write => format!("This {context} references a record that no longer exists."),
            Op::Delete => format!(
                "Cannot delete this {context} because it is still referenced by other records."
            ),
        };
    }

    // NOT NULL constraint failed: table.column
    if let Some(pos) = raw.find("NOT NULL constraint failed: ") {
        let rest = &raw[pos + "NOT NULL constraint failed: ".len()..];
        let rest = rest.trim_end_matches(|c: char| c == '"' || c == ')' || c.is_whitespace());
        let col = rest.split('.').last().unwrap_or("field").replace('_', " ");
        return format!("The {col} field is required.");
    }

    // Default: neutral verb so the message reads correctly with a noun context
    format!("Could not save {context}: {raw}")
}

/// Classify a transaction error into the right HTTP status. A not-found lookup
/// inside the closure — `DbErr::RecordNotFound`, produced by
/// [`crate::error::DbOptionExt::or_404_db`] — becomes a 404; a `DbErr::Custom`
/// (an already-friendly business-rule rejection raised inside the closure, e.g.
/// the lab/self dev mutual-exclusion guard) passes through verbatim as a 422;
/// every other error stays a friendly 422. The transactional handlers — both
/// creates (lab/self dev) and deletes (shots, lab/self dev) — use this; a
/// double-delete of a stale id returns NOT_FOUND, matching the non-transactional
/// handlers' `or_404`. `op` selects the FK wording (see [`Op`]): a transactional
/// create passes [`Op::Write`], a delete passes [`Op::Delete`]. The inner
/// messages are taken directly (not via `Display`) to avoid SeaORM's
/// "RecordNotFound Error: " / "Custom Error: " prefixes.
pub fn friendly_txn_err(context: &str, op: Op, e: TransactionError<DbErr>) -> AppError {
    match e {
        TransactionError::Transaction(DbErr::RecordNotFound(m)) => AppError::NotFound(m),
        TransactionError::Transaction(DbErr::Custom(m)) => AppError::UnprocessableEntity(m),
        other => AppError::UnprocessableEntity(friendly_err_op(context, op, other)),
    }
}

/// Classify a non-transactional delete error. The `delete` services signal an
/// already-deleted id by returning `DbErr::RecordNotFound` (when
/// `rows_affected == 0`); map that to a 404 so a stale-tab / double-tap delete
/// surfaces "already deleted" instead of a no-op 204, matching the transactional
/// delete handlers via [`friendly_txn_err`]. Every other `DbErr` (e.g. an FK
/// constraint) stays a friendly 422. The message is taken directly to avoid
/// SeaORM's "RecordNotFound Error: " prefix.
///
/// Delete-only by design: the FK wording is hardcoded to [`Op::Delete`] ("still
/// referenced by other records"). A create/update handler must NOT reuse this —
/// route writes through [`friendly_err`] (Op::Write) so an FK violation reads as
/// a missing reference, not a delete the user never performed (kammerz-956).
pub fn friendly_delete_err(context: &str, e: DbErr) -> AppError {
    match e {
        DbErr::RecordNotFound(m) => AppError::NotFound(m),
        other => AppError::UnprocessableEntity(friendly_err_op(context, Op::Delete, other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::RuntimeErr;

    // A FOREIGN KEY violation as SeaORM surfaces it: `DbErr::Exec` whose Display
    // is "Execution Error: <raw sqlite text>". We use this rather than
    // `DbErr::Custom` because `friendly_txn_err` intercepts `Custom` (treating it
    // as an already-friendly business message) before the FK branch is reached.
    fn fk_db_err() -> DbErr {
        DbErr::Exec(RuntimeErr::Internal(
            "error returned from database: (code: 787) FOREIGN KEY constraint failed".to_string(),
        ))
    }

    #[test]
    fn write_fk_violation_reads_as_missing_reference() {
        let msg = friendly_err("shot", fk_db_err());
        assert_eq!(msg, "This shot references a record that no longer exists.");
        assert!(!msg.to_lowercase().contains("delete"));
    }

    #[test]
    fn delete_fk_violation_keeps_referenced_wording() {
        let err = friendly_delete_err("film stock", fk_db_err());
        let AppError::UnprocessableEntity(msg) = err else {
            panic!("FK violation on delete should be a 422, got: {err:?}");
        };
        assert_eq!(
            msg,
            "Cannot delete this film stock because it is still referenced by other records."
        );
    }

    #[test]
    fn txn_fk_wording_follows_op() {
        let write = friendly_txn_err(
            "lab development",
            Op::Write,
            TransactionError::Transaction(fk_db_err()),
        );
        let AppError::UnprocessableEntity(write_msg) = write else {
            panic!("expected 422");
        };
        assert_eq!(
            write_msg,
            "This lab development references a record that no longer exists."
        );

        let delete = friendly_txn_err(
            "shot",
            Op::Delete,
            TransactionError::Transaction(fk_db_err()),
        );
        let AppError::UnprocessableEntity(delete_msg) = delete else {
            panic!("expected 422");
        };
        assert_eq!(
            delete_msg,
            "Cannot delete this shot because it is still referenced by other records."
        );
    }
}
