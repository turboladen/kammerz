use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use sea_orm::{DatabaseConnection, Set};
use serde::Deserialize;

use crate::AppState;
use crate::auth::middleware::RequireAuth;
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::extract::Json;
use crate::patch::{now_string, trim, trim_opt};
use crate::routes::friendly_err;
use crate::services::import_service::{ImportService, ModelInfo, ParsedRoll};
use crate::services::roll_service::{ImportDevRecord, ImportShotEntry, RollService};
use crate::services::settings_service::SettingsService;
use crate::validate::{
    require_nonempty, validate_date_opt, validate_non_negative_i32, validate_time,
};
use entity::roll::{self, PushPull};
use migration::backfilled_dates;

const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";

// --- DTOs (moved verbatim from commands/import.rs) ---

#[derive(Debug, Deserialize)]
pub struct ImportRollDto {
    pub roll_id: String,
    pub camera_id: Option<i32>,
    pub film_stock_id: Option<i32>,
    pub lens_id: Option<i32>,
    /// Legacy lifecycle status the import UI still sends. There is no stored
    /// status (ADR-0013); it is consumed only to backfill lifecycle dates so the
    /// imported roll derives to the intended activity state (kammerz-gsj6 tracks
    /// the dev-stage-status import gap).
    pub status: String,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub push_pull: Option<PushPull>,
    pub notes: Option<String>,
    pub shots: Vec<ImportShotDto>,
}

#[derive(Debug, Deserialize)]
pub struct ImportShotDto {
    pub frame_number: String,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub date: Option<String>,
    pub time: Option<String>,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub lens_ids: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct ParseNoteDto {
    pub note_text: String,
    pub model: Option<String>,
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/models", get(list_models))
        .route("/parse", post(parse_note))
        .route("/roll", post(import_parsed_roll))
}

// --- Key resolution ---

/// Resolve the Anthropic API key: prefer the server config (env
/// `ANTHROPIC_API_KEY`), then fall back to the `claude_api_key` settings row.
async fn resolve_key(db: &DatabaseConnection, config: &AppConfig) -> AppResult<String> {
    if let Some(k) = &config.anthropic_api_key {
        return Ok(k.clone());
    }
    SettingsService::get_setting(db, "claude_api_key")
        .await?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            AppError::UnprocessableEntity(
                "No Anthropic API key configured. Set it in Settings or the ANTHROPIC_API_KEY env var."
                    .into(),
            )
        })
}

// --- Handlers ---

async fn list_models(
    _: RequireAuth,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ModelInfo>>> {
    let key = resolve_key(&state.db, &state.config).await?;
    ImportService::list_models(&key)
        .await
        .map(Json)
        .map_err(AppError::UnprocessableEntity)
}

async fn parse_note(
    _: RequireAuth,
    State(state): State<AppState>,
    Json(data): Json<ParseNoteDto>,
) -> AppResult<Json<ParsedRoll>> {
    let key = resolve_key(&state.db, &state.config).await?;
    let model = match data.model {
        Some(m) if !m.is_empty() => m,
        _ => SettingsService::get_setting(&state.db, "claude_model")
            .await?
            .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
    };
    ImportService::parse_note(&key, &model, &data.note_text)
        .await
        .map(Json)
        .map_err(AppError::UnprocessableEntity)
}

async fn import_parsed_roll(
    _: RequireAuth,
    State(state): State<AppState>,
    Json(data): Json<ImportRollDto>,
) -> AppResult<(StatusCode, Json<i32>)> {
    validate_date_opt("date_loaded", &data.date_loaded)?;
    validate_date_opt("date_finished", &data.date_finished)?;
    // Server-side mirror of PR #75's client guards (kammerz-grd): a blank
    // roll_id otherwise satisfies NOT NULL and collides on the UNIQUE index as a
    // confusing duplicate; a negative frame_count is nonsensical.
    require_nonempty("roll_id", &data.roll_id)?;
    validate_non_negative_i32("frame_count", data.frame_count)?;
    for (i, s) in data.shots.iter().enumerate() {
        validate_date_opt(&format!("shots[{i}].date"), &s.date)?;
        validate_time(&format!("shots[{i}].time"), &s.time)?;
    }

    // Pre-validate frame numbers against the UNIQUE(roll_id, frame_number) index
    // the transaction would otherwise hit. The LLM prompt encourages "Same"-
    // propagation and range notation, so duplicates are plausible; surfacing them
    // here yields a targeted 422 naming the offending shot instead of the generic
    // constraint error mapped through `friendly_err`. Compare trimmed values, the
    // same form persisted below.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (i, s) in data.shots.iter().enumerate() {
        let frame = s.frame_number.trim();
        if frame.is_empty() {
            return Err(AppError::UnprocessableEntity(format!(
                "shots[{i}]: frame number is required"
            )));
        }
        if !seen.insert(frame) {
            return Err(AppError::UnprocessableEntity(format!(
                "shots[{i}]: duplicate frame number \"{frame}\""
            )));
        }
    }

    // Reject an unknown legacy status outright: `backfilled_dates` treats it as
    // rank-0 (a silent no-op), which would mask client drift/typos and import the
    // roll with an unintended derived lifecycle. The old `RollStatus` enum got
    // this 422 for free from serde; with the enum retired, enforce it here
    // against the same canonical value set the backfill ranks by.
    if !migration::BACKFILL_ORDER.contains(&data.status.as_str()) {
        return Err(AppError::UnprocessableEntity(format!(
            "unknown status \"{}\" — expected one of: {}",
            data.status,
            migration::BACKFILL_ORDER.join(", ")
        )));
    }

    // No stored status (ADR-0013): translate the legacy status the import UI
    // sends into lifecycle dates so the roll derives to the intended activity
    // state. Borrow only recorded dates (max shot date / provided dates) — never
    // fabricated (kammerz-gsj6: a dev-stage status with no dev record degrades).
    let max_shot_date = data
        .shots
        .iter()
        .filter_map(|s| s.date.as_deref().map(str::trim).filter(|d| !d.is_empty()))
        .max()
        .map(str::to_string);
    let date_loaded_in = data
        .date_loaded
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let date_finished_in = data
        .date_finished
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let filled = backfilled_dates(
        &data.status,
        date_loaded_in,
        date_finished_in,
        max_shot_date.as_deref(),
        None,
        None,
        None,
        None,
    );
    let date_finished_final = date_finished_in
        .map(str::to_string)
        .or(filled.date_finished);

    // Synthesize the dev record (dev-stage statuses) and tail dates (terminal
    // statuses) the roll needs to derive its intended activity state. `anchor` is
    // the best real recorded date to borrow as an honest LOWER BOUND for any
    // milestone the paper note doesn't date — finished-shooting ?? max shot date
    // ?? loaded. It is never a fabricated date. Consequence, disclosed to the user
    // at entry (import page hint) and in the PR: a completed-status import stamps
    // Scanned/Post-processed/Archived with this same shoot-era date, correctable on
    // the roll's activity board (kammerz-gsj6).
    let anchor = date_finished_final
        .as_deref()
        .or(max_shot_date.as_deref())
        .or(date_loaded_in);
    let life = import_lifecycle(&data.status, anchor);

    let now = now_string();

    let roll_model = roll::ActiveModel {
        roll_id: trim(data.roll_id),
        camera_id: Set(data.camera_id),
        film_stock_id: Set(data.film_stock_id),
        lens_id: Set(data.lens_id),
        frame_count: Set(data.frame_count),
        date_loaded: trim_opt(data.date_loaded),
        date_finished: Set(date_finished_final),
        date_scanned: Set(life.date_scanned),
        date_post_processed: Set(life.date_post_processed),
        date_archived: Set(life.date_archived),
        push_pull: Set(data.push_pull),
        notes: trim_opt(data.notes),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };

    let shot_entries: Vec<ImportShotEntry> = data
        .shots
        .into_iter()
        .map(|s| ImportShotEntry {
            frame_number: s.frame_number.trim().to_string(),
            aperture: s.aperture.map(|v| v.trim().to_string()),
            shutter_speed: s.shutter_speed.map(|v| v.trim().to_string()),
            date: s.date.map(|v| v.trim().to_string()),
            // `time` carries a stricter contract than its free-text siblings —
            // canonical HH:MM or NULL (see validate_time) — so collapse a
            // whitespace-only value to None, matching the create/update paths'
            // trim_opt rather than persisting an empty string.
            time: s.time.and_then(|v| {
                let t = v.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            }),
            location: s.location.map(|v| v.trim().to_string()),
            notes: s.notes.map(|v| v.trim().to_string()),
            lens_ids: s.lens_ids,
        })
        .collect();

    let id = RollService::import_roll(&state.db, roll_model, shot_entries, life.dev)
        .await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("roll", e)))?;
    Ok((StatusCode::CREATED, Json(id)))
}

/// What an import synthesizes for a legacy status beyond `date_finished`: the dev
/// record to create (dev-stage statuses) and the tail dates to set (terminal
/// statuses).
struct ImportLifecycle {
    dev: Option<ImportDevRecord>,
    date_scanned: Option<String>,
    date_post_processed: Option<String>,
    date_archived: Option<String>,
}

/// Map a legacy import status to the dev record + tail dates the roll needs to
/// derive its intended activity state (kammerz-gsj6). `anchor` is the honest
/// lower-bound date borrowed for milestones the note doesn't date (never
/// fabricated); when `None`, completion/tail dates stay unset and the status
/// degrades one step (documented per-status in `tests/import.rs`).
///
/// Terminal statuses (scanned/post-processed/archived) create NO dev record —
/// lab vs self is unknowable from a terminal status, and development derives
/// implicitly-done from any tail date (`activity.rs`). The tail three don't imply
/// each other, so each must be set explicitly up to the status's rank.
fn import_lifecycle(status: &str, anchor: Option<&str>) -> ImportLifecycle {
    let a = || anchor.map(str::to_string);
    let mut out = ImportLifecycle {
        dev: None,
        date_scanned: None,
        date_post_processed: None,
        date_archived: None,
    };
    match status {
        "at-lab" => {
            out.dev = Some(ImportDevRecord::Lab {
                date_received: None,
            })
        }
        "lab-done" => out.dev = Some(ImportDevRecord::Lab { date_received: a() }),
        "developing" => {
            out.dev = Some(ImportDevRecord::SelfDev {
                date_processed: None,
            })
        }
        "developed" => {
            out.dev = Some(ImportDevRecord::SelfDev {
                date_processed: a(),
            })
        }
        "scanned" => out.date_scanned = a(),
        "post-processed" => {
            out.date_scanned = a();
            out.date_post_processed = a();
        }
        "archived" => {
            out.date_scanned = a();
            out.date_post_processed = a();
            out.date_archived = a();
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_pre_dev_statuses_synthesize_nothing() {
        for s in ["loaded", "shooting", "shot"] {
            let l = import_lifecycle(s, Some("2026-01-05"));
            assert!(
                l.dev.is_none()
                    && l.date_scanned.is_none()
                    && l.date_post_processed.is_none()
                    && l.date_archived.is_none(),
                "{s} should synthesize no dev record or tail dates"
            );
        }
    }

    #[test]
    fn lifecycle_dev_stage_statuses_create_records() {
        assert_eq!(
            import_lifecycle("at-lab", Some("2026-01-05")).dev,
            Some(ImportDevRecord::Lab {
                date_received: None
            }),
            "at-lab: lab record in progress, no completion"
        );
        assert_eq!(
            import_lifecycle("lab-done", Some("2026-01-05")).dev,
            Some(ImportDevRecord::Lab {
                date_received: Some("2026-01-05".into())
            }),
            "lab-done: lab record completed via anchor"
        );
        assert_eq!(
            import_lifecycle("developing", Some("2026-01-05")).dev,
            Some(ImportDevRecord::SelfDev {
                date_processed: None
            }),
            "developing: self record in progress, no completion"
        );
        assert_eq!(
            import_lifecycle("developed", Some("2026-01-05")).dev,
            Some(ImportDevRecord::SelfDev {
                date_processed: Some("2026-01-05".into())
            }),
            "developed: self record completed via anchor"
        );
    }

    #[test]
    fn lifecycle_terminal_statuses_fill_tail_chain_up_to_rank() {
        let scanned = import_lifecycle("scanned", Some("2026-01-05"));
        assert_eq!(scanned.date_scanned.as_deref(), Some("2026-01-05"));
        assert!(scanned.date_post_processed.is_none() && scanned.date_archived.is_none());
        assert!(
            scanned.dev.is_none(),
            "terminal statuses create no dev record"
        );

        let pp = import_lifecycle("post-processed", Some("2026-01-05"));
        assert_eq!(pp.date_scanned.as_deref(), Some("2026-01-05"));
        assert_eq!(pp.date_post_processed.as_deref(), Some("2026-01-05"));
        assert!(pp.date_archived.is_none());

        let arch = import_lifecycle("archived", Some("2026-01-05"));
        assert_eq!(arch.date_scanned.as_deref(), Some("2026-01-05"));
        assert_eq!(arch.date_post_processed.as_deref(), Some("2026-01-05"));
        assert_eq!(arch.date_archived.as_deref(), Some("2026-01-05"));
    }

    #[test]
    fn lifecycle_absent_anchor_degrades_completions() {
        // Nothing recorded to borrow: completion/tail dates stay None. In-progress
        // dev states still get their (dateless) record — the honest floor.
        assert_eq!(
            import_lifecycle("lab-done", None).dev,
            Some(ImportDevRecord::Lab {
                date_received: None
            })
        );
        assert_eq!(
            import_lifecycle("developed", None).dev,
            Some(ImportDevRecord::SelfDev {
                date_processed: None
            })
        );
        let arch = import_lifecycle("archived", None);
        assert!(
            arch.date_scanned.is_none()
                && arch.date_post_processed.is_none()
                && arch.date_archived.is_none()
        );
    }
}
