# ADR-0013: Activity-based roll lifecycle — derived states replace the status enum

- **Status:** Accepted
- **Date:** 2026-07-18
- **Supersedes:** [ADR-0007](0007-data-driven-roll-status-sync.md) (data-driven
  status sync — its principle survives; its mechanism and the enum it reconciles
  are retired)
- **Related:** [ADR-0009](0009-negatives-pickup-derived-state.md) (prior art for
  derive-from-dates); [ADR-0008](0008-roll-detail-two-pane-activity-log.md) is
  _partially_ affected — the chevron status control it introduced is replaced by
  the activity board, while its append-only activity log survives unchanged;
  beads `kammerz-o4c8` (epic), `kammerz-b0ix`, `kammerz-64ga`, `kammerz-4she`,
  `kammerz-11o3`

## Context

The single `RollStatus` enum (`loaded` → … → `archived`) forces a linear story
onto a process whose tail is not linear. After development, three activities —
scanning, post-processing in Lightroom/Photos, archiving negatives — overlap
and reorder freely: post-processing can begin mid-scan and take days; negatives
can be archived before editing, or never (a lab discards them), which the enum
cannot represent at all. The flow also has no way to show waiting ("negs dry,
not yet scanning") or in-progress work — its past-tense labels only describe
completed snapshots. Finally, a stored status can contradict the data that
justifies it; ADR-0007's reconciliation machinery (and the orphaned-status bugs
it kept patching — `kammerz-afc`, `kammerz-e2u`, `kammerz-3wg`) exists to fight
exactly that drift.

## Decision

**A roll's lifecycle is five activities whose states are derived from date
presence; there is no stored status.**

- Activities: **Shooting**, **Development** (the existing lab/self dev record —
  lab-vs-self is a property of the activity), **Scanning**,
  **Post-processing**, **Archiving**.
- Duration activities derive: start date set + no completion → _in progress_;
  completion set → _done_; neither → _not started_. Archiving is a moment:
  _done_ (`date_archived`, optional location) / _**N/A**_ (`archive_na`,
  optional reason — the no-negatives case) / _not done_.
- **Implicit completion:** an activity also counts as done when a strictly
  later activity has any date (shooting → development → each of the tail
  three). The tail three never imply each other — they genuinely overlap.
- **Waiting is derived, never stored:** development done + scanning not
  started renders "To scan"; no explicit waiting states exist.
- The backend derives everything consumers need — per-activity states, a
  compound badge of _all_ in-progress activities ("Scanning ·
  Post-processing"), waiting labels, and an earliest-incomplete scalar for
  grouping/sorting — and returns it on roll responses. The frontend never
  re-derives.
- The `status` column, `RollStatus` enum, `LAB_FLOW`/`SELF_FLOW`,
  `status-flows.json`, and `tests/status_flows.rs` fixture cross-check are all
  retired. Migration adds the few missing columns and backfills only from
  recorded dates, never fabricated ones.
- ADR-0007's surviving rules, restated in activity terms: the backend owns
  derivation; backward moves are date-clears behind a confirmation; starting
  Development routes through the dev-record dialog whose dates drive the
  activity's state.

## Consequences

- **Positive:** the orphaned-status bug class is eliminated _by construction_ —
  with no stored status there is nothing to reconcile, so ADR-0007's sync
  primitives, create-vs-edit adoption asymmetry, and fixture-drift gates all
  disappear rather than needing maintenance.
- **Positive:** the model finally represents reality — overlapping tail work,
  waiting gaps, archive-before-edit, and archive-N/A — and the past-tense
  label problem dissolves because labels describe activity states, not
  pipeline snapshots.
- **Negative:** "which rolls are at status X" becomes a derived query; every
  status consumer (dashboard sections, rolls list grouping, search, stats,
  import, Badge) must move to the server-computed phase fields in one
  coordinated API change (`kammerz-b0ix`).
- **Negative / limits:** derivation from date presence means a wrong date
  silently changes state; the mitigation is the activity board making every
  driving date visible and editable in place. Compound badges are wider than
  enum pills, and grouping needs the separate scalar key to stay
  deterministic.
