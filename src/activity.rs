//! Activity-based roll lifecycle derivation (ADR-0013).
//!
//! A roll's lifecycle is five activities — **shooting**, **development**,
//! **scanning**, **post-processing**, **archiving** — whose states are derived
//! purely from date presence. There is no stored status. This module is the
//! single source of truth for that derivation: the backend computes per-activity
//! states, a compound badge, a group/sort key, and a `done` flag here, and every
//! consumer (roll list/detail, search, stats, development lists) runs its rows
//! through [`derive`]. The frontend never re-derives.
//!
//! Pure and DB-free so it is exhaustively unit-testable and reused across the
//! request handlers without a database round-trip. This module's unit tests are
//! the single home for the derivation's coverage (there is no fixture cross-check
//! and no compat status string anymore — kammerz-1ezf retired both).

use serde::Serialize;

/// Canonical activity order. A kind's index in this array is its group/sort key.
pub const ACTIVITY_KINDS: [&str; 5] = [
    "shooting",
    "development",
    "scanning",
    "post_processing",
    "archiving",
];

// NOTE: phase LABELS deliberately live only in the frontend (`phase.ts`
// PHASE_META). The backend speaks `group_key` integers across the wire — a
// cross-language label-string contract would be unguardable drift (the exact
// disease the retired status-flows fixture existed to contain).

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

impl ActivitySignals {
    /// Apply the lab/self dev triplet from raw query columns. The selection —
    /// lab takes precedence for legacy both-dev rolls; the completion date comes
    /// from the chosen record — lives HERE so no consumer re-implements it and
    /// drifts (search/stats/roll-list all route through this).
    pub fn with_dev(
        mut self,
        lab_dev_id: Option<i32>,
        lab_completion: Option<String>,
        self_dev_id: Option<i32>,
        self_completion: Option<String>,
    ) -> Self {
        let is_lab_dev = lab_dev_id.is_some();
        self.has_dev = is_lab_dev || self_dev_id.is_some();
        self.is_lab_dev = is_lab_dev;
        self.dev_completion = if is_lab_dev {
            lab_completion
        } else {
            self_completion
        };
        self
    }
}

/// The per-activity resolution flags in canonical order. Resolved = done, or
/// (archiving only) N/A. [`derive`] destructures these for its activity states
/// and [`group_key`] positions on them — one definition, so the badge/states
/// and the group key can never disagree. The implicit-completion chain
/// (shooting -> development -> tail; a dev record or any tail date completes
/// its predecessors; the tail three never imply each other) lives HERE.
fn resolved_flags(sig: &ActivitySignals) -> [bool; 5] {
    let any_tail_date = sig.scan_started.is_some()
        || sig.date_scanned.is_some()
        || sig.post_processing_started.is_some()
        || sig.date_post_processed.is_some()
        || sig.date_archived.is_some();
    [
        sig.date_finished.is_some() || sig.has_dev || any_tail_date,
        sig.dev_completion.is_some() || any_tail_date,
        sig.date_scanned.is_some(),
        sig.date_post_processed.is_some(),
        sig.date_archived.is_some() || sig.archive_na,
    ]
}

/// The group/sort key alone — for consumers that need no badge or activity
/// states (the stats phase tally). Skips [`derive`]'s Vec/String allocations.
pub fn group_key(sig: &ActivitySignals) -> i32 {
    resolved_flags(sig)
        .iter()
        .position(|r| !r)
        .map_or(5, |i| i as i32)
}

/// Derive the full activity view for one roll from its signals.
///
/// Every signal column MUST be populated from the query row — a partial signal
/// set silently derives a different phase/badge than the canonical roll list
/// (kammerz-1ezf review). `dev_started` is the only display-only exception
/// (it feeds `Activity.start`, never states/badge/group_key), and shooting's
/// `shot_count`/`date_loaded` only matter while shooting can still be
/// in-progress (impossible once a dev record exists).
pub fn derive(sig: &ActivitySignals) -> RollActivity {
    // The per-activity done flags come from the SAME resolved_flags source as
    // group_key — destructured, not recomputed — so the activity states/badge
    // and the group key cannot drift apart. (resolved_flags[4] is
    // "archiving RESOLVED" — done OR N/A — so archiving's done flag is read
    // from its own date below to keep the done-vs-na distinction.)
    let [
        shooting_done,
        development_done,
        scanning_done,
        pp_done,
        _archiving_resolved,
    ] = resolved_flags(sig);
    let development_started = sig.has_dev;
    let shooting_started = sig.date_loaded.is_some() || sig.shot_count > 0;
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

    let group_key = group_key(sig);
    let done = group_key == 5;

    let badge = badge_for(&activities, done, group_key);

    RollActivity {
        activities,
        badge,
        group_key,
        done,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn state(ra: &RollActivity, kind: &str) -> &'static str {
        ra.activities.iter().find(|a| a.kind == kind).unwrap().state
    }

    // --- derivation across the lifecycle (group_key / badge / activity states) ---
    // These walk the milestones the retired compat `status` used to name, now
    // expressed purely in activity terms. Note the intended collapse: lab vs self
    // is NOT visible in the derived view (both develop → group_key 1 badge
    // "Developing"; both complete → group_key 2 badge "To scan") — the lab/self
    // distinction lives on the dev record, not the lifecycle position (ADR-0013).

    #[test]
    fn empty_roll_is_loaded() {
        let ra = derive(&ActivitySignals::default());
        assert_eq!(ra.group_key, 0);
        assert_eq!(ra.badge, "Loaded");
        assert_eq!(state(&ra, "shooting"), "not_started");
    }

    #[test]
    fn date_loaded_only_starts_shooting() {
        let ra = derive(&ActivitySignals {
            date_loaded: Some("2026-01-01".into()),
            ..Default::default()
        });
        // Loaded (shooting started, no shots yet) — shooting is in progress.
        assert_eq!(state(&ra, "shooting"), "in_progress");
        assert_eq!(ra.group_key, 0);
        assert_eq!(ra.badge, "Shooting");
    }

    #[test]
    fn shots_keep_shooting_in_progress() {
        let ra = derive(&ActivitySignals {
            shot_count: 3,
            date_loaded: Some("2026-01-01".into()),
            ..Default::default()
        });
        assert_eq!(state(&ra, "shooting"), "in_progress");
        assert_eq!(ra.group_key, 0);
        assert_eq!(ra.badge, "Shooting");
    }

    #[test]
    fn finished_shooting_no_dev_waits_to_develop() {
        let ra = derive(&ActivitySignals {
            shot_count: 36,
            date_loaded: Some("2026-01-01".into()),
            date_finished: Some("2026-01-05".into()),
            ..Default::default()
        });
        assert_eq!(state(&ra, "shooting"), "done");
        assert_eq!(state(&ra, "development"), "not_started");
        assert_eq!(ra.group_key, 1);
        assert_eq!(ra.badge, "To develop");
    }

    #[test]
    fn dev_record_in_progress_then_done_lab() {
        let developing = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: true,
            ..Default::default()
        });
        assert_eq!(state(&developing, "shooting"), "done"); // implicit: dev exists
        assert_eq!(state(&developing, "development"), "in_progress");
        assert_eq!(developing.group_key, 1);
        assert_eq!(developing.badge, "Developing");

        let developed = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: true,
            dev_completion: Some("2026-01-10".into()),
            ..Default::default()
        });
        assert_eq!(state(&developed, "development"), "done");
        assert_eq!(developed.group_key, 2);
        assert_eq!(developed.badge, "To scan");
    }

    #[test]
    fn self_dev_derives_same_view_as_lab() {
        // The collapse, asserted directly: a self dev derives the SAME group_key /
        // badge / development state as a lab dev at the same completion. The
        // lab-vs-self distinction is only on the dev record.
        let self_developing = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: false,
            ..Default::default()
        });
        assert_eq!(state(&self_developing, "development"), "in_progress");
        assert_eq!(self_developing.group_key, 1);
        assert_eq!(self_developing.badge, "Developing");

        let self_developed = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: false,
            dev_completion: Some("2026-01-10".into()),
            ..Default::default()
        });
        assert_eq!(state(&self_developed, "development"), "done");
        assert_eq!(self_developed.group_key, 2);
        assert_eq!(self_developed.badge, "To scan");
    }

    #[test]
    fn with_dev_prefers_lab_when_both_present() {
        // The documented lab-precedence invariant (relied on by roll/stats/search
        // services): a legacy both-dev roll derives from the LAB record — is_lab_dev
        // true, dev_completion == the lab date (NOT the self date) — and the derived
        // view reflects the lab completion (development done → group_key 2, "To scan").
        let sig = ActivitySignals::default().with_dev(
            Some(1),
            Some("2026-01-10".into()), // lab completion
            Some(2),
            Some("2026-02-20".into()), // self completion (must be ignored)
        );
        assert!(sig.has_dev);
        assert!(sig.is_lab_dev);
        assert_eq!(sig.dev_completion.as_deref(), Some("2026-01-10"));

        let ra = derive(&sig);
        assert_eq!(state(&ra, "development"), "done");
        assert_eq!(ra.group_key, 2);
        assert_eq!(ra.badge, "To scan");
    }

    #[test]
    fn scanned_marks_scanning_done() {
        let ra = derive(&ActivitySignals {
            has_dev: true,
            is_lab_dev: true,
            dev_completion: Some("2026-01-10".into()),
            date_scanned: Some("2026-01-12".into()),
            ..Default::default()
        });
        assert_eq!(state(&ra, "scanning"), "done");
        assert_eq!(ra.group_key, 3);
        assert_eq!(ra.badge, "To edit");
    }

    #[test]
    fn scanned_no_dev_implicitly_completes_development() {
        // Scanned with no dev record (the retired "undecided" path): development
        // is implicitly done via the later scan date.
        let ra = derive(&ActivitySignals {
            shot_count: 12,
            date_finished: Some("2026-01-05".into()),
            date_scanned: Some("2026-01-12".into()),
            ..Default::default()
        });
        assert_eq!(state(&ra, "development"), "done");
        assert_eq!(state(&ra, "shooting"), "done");
        assert_eq!(state(&ra, "scanning"), "done");
        assert_eq!(ra.group_key, 3);
    }

    #[test]
    fn post_processed_then_archived() {
        let pp = derive(&ActivitySignals {
            date_scanned: Some("2026-01-12".into()),
            date_post_processed: Some("2026-01-14".into()),
            ..Default::default()
        });
        assert_eq!(state(&pp, "post_processing"), "done");
        assert_eq!(pp.group_key, 4);
        assert_eq!(pp.badge, "To archive");

        let archived = derive(&ActivitySignals {
            date_scanned: Some("2026-01-12".into()),
            date_post_processed: Some("2026-01-14".into()),
            date_archived: Some("2026-01-20".into()),
            ..Default::default()
        });
        assert_eq!(state(&archived, "archiving"), "done");
        assert!(archived.done);
        assert_eq!(archived.group_key, 5);
        assert_eq!(archived.badge, "Done");
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
