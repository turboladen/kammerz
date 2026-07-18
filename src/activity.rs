//! Activity-based roll lifecycle derivation (ADR-0013).
//!
//! A roll's lifecycle is five activities — **shooting**, **development**,
//! **scanning**, **post-processing**, **archiving** — whose states are derived
//! purely from date presence. There is no stored status. This module is the
//! single source of truth for that derivation: the backend computes per-activity
//! states, a compound badge, a group/sort key, a `done` flag, AND a *compat*
//! legacy `status` string here, and every consumer (roll list/detail, search,
//! stats, development lists) runs its rows through [`derive`]. The frontend never
//! re-derives.
//!
//! Pure and DB-free so it is exhaustively unit-testable and reused across the
//! request handlers without a database round-trip.
//!
//! The compat `status` is a transitional shim (kammerz-1ezf tracks its removal
//! once the remaining frontend consumers move to the activity fields): it lets
//! the existing frontend keep reading `roll.status` unchanged while the new
//! `activities`/`badge`/`group_key` fields ride alongside for the activity-board
//! UI (kammerz-64ga/4she). It is derived fresh from data on every read, so unlike
//! the retired stored enum it can never drift from the dates that justify it.

use serde::Serialize;

/// Canonical activity order. A kind's index in this array is its group/sort key.
pub const ACTIVITY_KINDS: [&str; 5] = [
    "shooting",
    "development",
    "scanning",
    "post_processing",
    "archiving",
];

/// One activity's derived state, with the dates that drive it (for the board UI).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Activity {
    /// One of [`ACTIVITY_KINDS`].
    pub kind: &'static str,
    /// `"not_started"` | `"in_progress"` | `"done"` | `"na"` (`na` only archiving).
    pub state: &'static str,
    pub start: Option<String>,
    pub completion: Option<String>,
}

/// Everything the frontend needs to render a roll's lifecycle without re-deriving.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RollActivity {
    /// The five activities in canonical order.
    pub activities: Vec<Activity>,
    /// Compound label of all in-progress activities ("Scanning · Post-processing"),
    /// else a waiting label from the earliest unresolved activity, else "Done".
    pub badge: String,
    /// Index of the earliest unresolved activity (0..=4), or 5 when the roll is Done.
    /// The scalar used for list group-by, dashboard ordering, and stats.
    pub group_key: i32,
    /// True when all five activities are resolved (archiving N/A counts as resolved).
    pub done: bool,
    /// Compat legacy status string (kammerz-1ezf). Derived from the same signals.
    pub status: String,
}

/// Raw per-roll date/record signals fed to [`derive`]. Callers populate this from
/// their query rows; every field defaults to "absent" so partial callers (e.g. the
/// dev-list, where a dev record always exists) can set only what they have.
#[derive(Debug, Clone, Default)]
pub struct ActivitySignals {
    pub shot_count: i64,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    /// A development record (lab or self) exists for the roll.
    pub has_dev: bool,
    /// True when that dev record is a lab dev (picks lab-done/at-lab vs developed/developing).
    pub is_lab_dev: bool,
    /// The dev record's start date, if any (lab `date_dropped_off`); display-only.
    pub dev_started: Option<String>,
    /// The dev record's completion date: lab `date_received` or self `date_processed`.
    pub dev_completion: Option<String>,
    pub scan_started: Option<String>,
    pub date_scanned: Option<String>,
    pub post_processing_started: Option<String>,
    pub date_post_processed: Option<String>,
    pub date_archived: Option<String>,
    pub archive_na: bool,
}

/// Derive the full activity view for one roll from its signals.
pub fn derive(sig: &ActivitySignals) -> RollActivity {
    // Which tail activities carry any date. The tail three overlap and never
    // imply each other, so each is judged only by its own dates.
    let scanning_has_date = sig.scan_started.is_some() || sig.date_scanned.is_some();
    let pp_has_date = sig.post_processing_started.is_some() || sig.date_post_processed.is_some();
    let any_tail_date = scanning_has_date || pp_has_date || sig.date_archived.is_some();

    // Implicit completion, walking the chain shooting -> development -> tail: an
    // activity is done when a strictly-later activity has a date. We treat an
    // existing dev record (`has_dev`) as evidence shooting finished — you do not
    // develop a roll you are still shooting.
    let development_done = sig.dev_completion.is_some() || any_tail_date;
    let development_started = sig.has_dev;
    let shooting_done = sig.date_finished.is_some() || sig.has_dev || any_tail_date;
    let shooting_started = sig.date_loaded.is_some() || sig.shot_count > 0;

    let scanning_done = sig.date_scanned.is_some();
    let pp_done = sig.date_post_processed.is_some();
    let archiving_done = sig.date_archived.is_some();

    let shooting = Activity {
        kind: "shooting",
        state: state_of(shooting_done, shooting_started),
        start: sig.date_loaded.clone(),
        completion: sig.date_finished.clone(),
    };
    let development = Activity {
        kind: "development",
        state: state_of(development_done, development_started),
        start: sig.dev_started.clone(),
        completion: sig.dev_completion.clone(),
    };
    let scanning = Activity {
        kind: "scanning",
        state: state_of(scanning_done, sig.scan_started.is_some() || scanning_done),
        start: sig.scan_started.clone(),
        completion: sig.date_scanned.clone(),
    };
    let post_processing = Activity {
        kind: "post_processing",
        state: state_of(pp_done, sig.post_processing_started.is_some() || pp_done),
        start: sig.post_processing_started.clone(),
        completion: sig.date_post_processed.clone(),
    };
    // Archiving is a moment, not a duration: done / N/A / not started (never in
    // progress). N/A and done are mutually exclusive; done wins if both are set.
    let archiving = Activity {
        kind: "archiving",
        state: if archiving_done {
            "done"
        } else if sig.archive_na {
            "na"
        } else {
            "not_started"
        },
        start: None,
        completion: sig.date_archived.clone(),
    };

    let activities = vec![shooting, development, scanning, post_processing, archiving];

    // Resolved = done, or (archiving only) N/A. group_key is the first unresolved.
    let resolved = [
        shooting_done,
        development_done,
        scanning_done,
        pp_done,
        archiving_done || sig.archive_na,
    ];
    let group_key = resolved.iter().position(|r| !r).map_or(5, |i| i as i32);
    let done = group_key == 5;

    let badge = badge_for(&activities, done, group_key);
    let status = legacy_status(sig);

    RollActivity {
        activities,
        badge,
        group_key,
        done,
        status,
    }
}

fn state_of(done: bool, started: bool) -> &'static str {
    if done {
        "done"
    } else if started {
        "in_progress"
    } else {
        "not_started"
    }
}

/// Human label for an in-progress activity, in the compound badge.
fn in_progress_label(kind: &str) -> &'static str {
    match kind {
        "shooting" => "Shooting",
        "development" => "Developing",
        "scanning" => "Scanning",
        "post_processing" => "Post-processing",
        _ => "",
    }
}

/// Waiting label for the earliest unresolved activity when nothing is in progress.
fn waiting_label(group_key: i32) -> &'static str {
    match group_key {
        0 => "Loaded",
        1 => "To develop",
        2 => "To scan",
        3 => "To edit",
        4 => "To archive",
        _ => "Done",
    }
}

fn badge_for(activities: &[Activity], done: bool, group_key: i32) -> String {
    let in_progress: Vec<&'static str> = activities
        .iter()
        .filter(|a| a.state == "in_progress")
        .map(|a| in_progress_label(a.kind))
        .collect();
    if !in_progress.is_empty() {
        in_progress.join(" · ")
    } else if done {
        "Done".to_string()
    } else {
        waiting_label(group_key).to_string()
    }
}

/// The compat legacy status string (highest applicable milestone wins). This
/// reproduces the semantics the retired `auto_sync`/`sync_*` machinery computed,
/// so every current frontend consumer keeps rendering unchanged. Rolls whose old
/// *stored* status had drifted from their data now render the data-consistent
/// value by design (ADR-0013).
///
/// `pub` so consumers that need only the compat status (search, stats, dev
/// lists) skip [`derive`]'s full activity build (a 5-element `Vec` + badge).
pub fn legacy_status(sig: &ActivitySignals) -> String {
    if sig.date_archived.is_some() {
        "archived"
    } else if sig.date_post_processed.is_some() {
        "post-processed"
    } else if sig.date_scanned.is_some() {
        "scanned"
    } else if sig.has_dev {
        match (sig.is_lab_dev, sig.dev_completion.is_some()) {
            (true, true) => "lab-done",
            (true, false) => "at-lab",
            (false, true) => "developed",
            (false, false) => "developing",
        }
    } else if sig.date_finished.is_some() {
        "shot"
    } else if sig.shot_count > 0 {
        "shooting"
    } else {
        "loaded"
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(ra: &RollActivity, kind: &str) -> &'static str {
        ra.activities.iter().find(|a| a.kind == kind).unwrap().state
    }

    // --- compat legacy status, all ten values ---

    #[test]
    fn status_loaded_when_empty() {
        let ra = derive(&ActivitySignals::default());
        assert_eq!(ra.status, "loaded");
        assert_eq!(ra.group_key, 0);
    }

    #[test]
    fn status_loaded_with_date_loaded_only() {
        let ra = derive(&ActivitySignals {
            date_loaded: Some("2026-01-01".into()),
            ..Default::default()
        });
        assert_eq!(ra.status, "loaded");
        // Shooting has started (loaded) but no shots yet.
        assert_eq!(state(&ra, "shooting"), "in_progress");
    }

    #[test]
    fn status_shooting_with_shots() {
        let ra = derive(&ActivitySignals {
            shot_count: 3,
            date_loaded: Some("2026-01-01".into()),
            ..Default::default()
        });
        assert_eq!(ra.status, "shooting");
    }

    #[test]
    fn status_shot_when_finished_no_dev() {
        let ra = derive(&ActivitySignals {
            shot_count: 36,
            date_loaded: Some("2026-01-01".into()),
            date_finished: Some("2026-01-05".into()),
            ..Default::default()
        });
        assert_eq!(ra.status, "shot");
        assert_eq!(state(&ra, "shooting"), "done");
        assert_eq!(state(&ra, "development"), "not_started");
        assert_eq!(ra.group_key, 1);
        assert_eq!(ra.badge, "To develop");
    }

    #[test]
    fn status_at_lab_and_lab_done() {
        let at_lab = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: true,
            ..Default::default()
        });
        assert_eq!(at_lab.status, "at-lab");
        assert_eq!(state(&at_lab, "shooting"), "done"); // implicit: dev exists
        assert_eq!(state(&at_lab, "development"), "in_progress");

        let lab_done = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: true,
            dev_completion: Some("2026-01-10".into()),
            ..Default::default()
        });
        assert_eq!(lab_done.status, "lab-done");
        assert_eq!(state(&lab_done, "development"), "done");
        assert_eq!(lab_done.badge, "To scan");
    }

    #[test]
    fn status_developing_and_developed() {
        let developing = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: false,
            ..Default::default()
        });
        assert_eq!(developing.status, "developing");

        let developed = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: false,
            dev_completion: Some("2026-01-10".into()),
            ..Default::default()
        });
        assert_eq!(developed.status, "developed");
    }

    #[test]
    fn status_scanned_requires_recorded_date() {
        // A `scanned` roll must actually carry date_scanned — a date-less roll
        // degrades (see status_shot_...). Here the date is present.
        let ra = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: true,
            dev_completion: Some("2026-01-10".into()),
            date_scanned: Some("2026-01-12".into()),
            ..Default::default()
        });
        assert_eq!(ra.status, "scanned");
        assert_eq!(state(&ra, "scanning"), "done");
    }

    #[test]
    fn status_scanned_undecided_path_no_dev() {
        // Scanned with no dev record (the retired "undecided" path): development
        // is implicitly done via the later scan date.
        let ra = derive(&ActivitySignals {
            shot_count: 12,
            date_finished: Some("2026-01-05".into()),
            date_scanned: Some("2026-01-12".into()),
            ..Default::default()
        });
        assert_eq!(ra.status, "scanned");
        assert_eq!(state(&ra, "development"), "done");
        assert_eq!(state(&ra, "shooting"), "done");
    }

    #[test]
    fn status_post_processed_and_archived() {
        let pp = derive(&ActivitySignals {
            date_scanned: Some("2026-01-12".into()),
            date_post_processed: Some("2026-01-14".into()),
            ..Default::default()
        });
        assert_eq!(pp.status, "post-processed");

        let archived = derive(&ActivitySignals {
            date_scanned: Some("2026-01-12".into()),
            date_post_processed: Some("2026-01-14".into()),
            date_archived: Some("2026-01-20".into()),
            ..Default::default()
        });
        assert_eq!(archived.status, "archived");
    }

    // --- implicit completion nuances ---

    #[test]
    fn post_processing_started_does_not_complete_scanning() {
        // The tail three overlap: editing mid-scan must not mark scanning done.
        let ra = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: false,
            dev_completion: Some("2026-01-10".into()),
            post_processing_started: Some("2026-01-13".into()),
            ..Default::default()
        });
        assert_eq!(state(&ra, "scanning"), "not_started");
        assert_eq!(state(&ra, "post_processing"), "in_progress");
    }

    #[test]
    fn compound_badge_lists_all_in_progress() {
        let ra = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: false,
            dev_completion: Some("2026-01-10".into()),
            scan_started: Some("2026-01-12".into()),
            post_processing_started: Some("2026-01-13".into()),
            ..Default::default()
        });
        assert_eq!(state(&ra, "scanning"), "in_progress");
        assert_eq!(state(&ra, "post_processing"), "in_progress");
        assert_eq!(ra.badge, "Scanning · Post-processing");
    }

    // --- archiving: done / N/A / done-wins ---

    #[test]
    fn archive_na_counts_as_resolved() {
        let ra = derive(&ActivitySignals {
            date_scanned: Some("2026-01-12".into()),
            date_post_processed: Some("2026-01-14".into()),
            archive_na: true,
            ..Default::default()
        });
        assert_eq!(state(&ra, "archiving"), "na");
        assert!(ra.done);
        assert_eq!(ra.group_key, 5);
        assert_eq!(ra.badge, "Done");
        // Compat: N/A has no legacy equivalent; the roll reads post-processed.
        assert_eq!(ra.status, "post-processed");
    }

    #[test]
    fn archive_done_wins_over_na() {
        let ra = derive(&ActivitySignals {
            date_archived: Some("2026-01-20".into()),
            archive_na: true,
            ..Default::default()
        });
        assert_eq!(state(&ra, "archiving"), "done");
    }

    #[test]
    fn fully_done_roll() {
        let ra = derive(&ActivitySignals {
            shot_count: 36,
            date_loaded: Some("2026-01-01".into()),
            date_finished: Some("2026-01-05".into()),
            has_dev: true,
            is_lab_dev: true,
            dev_completion: Some("2026-01-10".into()),
            date_scanned: Some("2026-01-12".into()),
            date_post_processed: Some("2026-01-14".into()),
            date_archived: Some("2026-01-20".into()),
            ..Default::default()
        });
        assert!(ra.done);
        assert_eq!(ra.group_key, 5);
        assert_eq!(ra.badge, "Done");
        assert_eq!(ra.status, "archived");
        for kind in ACTIVITY_KINDS {
            assert_eq!(state(&ra, kind), "done", "{kind} should be done");
        }
    }

    #[test]
    fn group_key_points_at_earliest_gap() {
        // Dev done, scanning skipped, post-processing done: earliest unresolved
        // is scanning (index 2).
        let ra = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: true,
            dev_completion: Some("2026-01-10".into()),
            date_post_processed: Some("2026-01-14".into()),
            ..Default::default()
        });
        assert_eq!(state(&ra, "scanning"), "not_started");
        assert_eq!(ra.group_key, 2);
        assert_eq!(ra.badge, "To scan");
    }
}
