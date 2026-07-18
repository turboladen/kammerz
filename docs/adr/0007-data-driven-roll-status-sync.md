# ADR-0007: Data-driven roll status — auto-sync from milestone dates + dev records

- **Status:** Superseded by [ADR-0013](0013-activity-based-roll-lifecycle.md)
- **Date:** 2026-06-03
- **Related:** `.claude/rules/domain-conventions.md` ("Data-driven status sync"),
  `src/services/roll_service.rs` (`auto_sync_status`, `advance_status_along`,
  `sync_lab_dev_status`/`sync_self_dev_status`, `create_synced_*_dev_status`,
  `resync_*_dev_status`), beads `kammerz-afc`, `kammerz-e2u`, `kammerz-3wg`,
  `kammerz-9xg`, `kammerz-a7e`, `kammerz-6ih`, `kammerz-mon`

## Context

A roll's status (`loaded` → … → `archived`) used to be a plain field the user set
directly by clicking a chevron, with lifecycle dates stamped separately (or not at
all — e.g. `date_received` had no capture point). This produced two problems: status
and the data that justified it (shots, lab/self dev records, milestone dates) could
drift apart, and the same status had inconsistent rules depending on which UI path
set it. A companion decision — letting the user pick the _actual_ date a milestone
happened, rather than silently stamping "today" — needed a single, durable source of
truth to key off, and clicking a chevron manually isn't it. The **fuzzy-date**
question this same spec raised is a separate, already-settled decision (see
[ADR-0004](0004-remove-fuzzy-dates.md)) and is out of scope here.

## Decision

**Roll status is derived data, not a direct-write field.** The backend — never a
frontend `$effect` — owns reconciling status from the presence of shots, lab/self
development records, and their milestone dates, transactionally, on every mutation
that could change the picture.

Two primitives in `roll_service.rs` implement this:

- `auto_sync_status(from_statuses, to_status)` — a conditional **set**: if the roll's
  current status is in an explicit `from` set, jump it to `to_status`. Used for
  simple triggers (first shot added → `shooting`; all shots deleted → `loaded`; a dev
  record deleted → back to `shot`).
- `advance_status_along(flow, target)` (via `sync_lab_dev_status`/`sync_self_dev_status`,
  built on the `LAB_FLOW`/`SELF_FLOW` orderings) — **forward-only** reconciliation:
  moves the roll to `target` if its current status sits at any _earlier_ rung of that
  flow, so a roll orphaned mid-path (e.g. imported at `at-lab` with no lab record) is
  fixed in one action, not several (`kammerz-afc`). It never moves backward and no-ops
  off-flow.

Rules that fall out of these primitives:

- **Create is data-driven and adopts orphans.** Creating a lab dev record advances to
  `lab-done` if `date_received` is already set, else `at-lab`; creating a self dev
  record advances to `developed` if `date_processed` is set, else `developing`. If the
  roll is stranded on the _sibling_ path with no record justifying it, the create-only
  helpers (`create_synced_lab_dev_status`/`create_synced_self_dev_status`) adopt it
  onto the new path — but only when no sibling dev record exists, so legitimate
  both-dev-type data is never yanked off its status (`kammerz-e2u`).
- **Edit resyncs only when the driving date's presence changes.** A dev-record edit
  calls `resync_lab_dev_status`/`resync_self_dev_status`, which advance forward the
  same way but never perform cross-flow adoption (that's create-only — editing must
  not relocate a roll the user deliberately positioned) and only revert
  (`lab-done`→`at-lab`, `developed`→`developing`) when the completing date is cleared.
  An edit that merely echoes a still-set date (e.g. changing unrelated notes) must not
  silently undo a confirmed backward move (`kammerz-3wg`).
- **Manual backward moves require explicit confirmation** (`ConfirmDialog` in the
  chevron UI) — the one path where the user is allowed to contradict the data state on
  purpose. Editing a Timeline date, by contrast, never changes status by itself; it's
  the dev-record/shot mutations layered on top of a date write that trigger sync.
- **Silent auto-advances get a visible signal.** Because the sync can move status
  without an explicit chevron click, the frontend surfaces a transient inline notice
  after reload when the fetched status differs from what the user clicked
  (`kammerz-9xg`); per-chevron hint text must mirror the click-handler's branch order
  exactly so it never lies (`kammerz-6ih`).
- **Ownership is exclusively backend.** All of the above lives in `roll_service.rs`
  and is invoked from the create/update/delete route handlers inside their
  transactions; the frontend never runs this logic in an `$effect` — it only renders
  the result and reacts to date-edit/dev-dialog user actions.

## Consequences

- **Positive:** status can never contradict the data that justifies it except through
  an explicit, confirmed backward click — the common failure mode (stale status after
  editing a date or importing historical data) is eliminated by construction.
- **Positive:** the two-primitive design (`auto_sync_status` conditional-set vs.
  `advance_status_along` forward-reconcile) keeps simple triggers and orphan-recovery
  logic composable rather than duplicated per call site; `LAB_FLOW`/`SELF_FLOW` are
  the single ordering source, cross-checked against the frontend fixture by
  `tests/status_flows.rs` (`kammerz-mon`).
- **Negative / limits:** the create-vs-edit asymmetry (adoption only on create, never
  on resync) and the "date-presence-changed" resync guard are subtle invariants that
  future dev-record-touching code must preserve deliberately — they were each added
  after a real bug (`kammerz-e2u`, `kammerz-3wg`), not designed upfront.
- **Negative:** auto-advance is inherently a little "invisible" — without the
  post-mutation notice (`kammerz-9xg`) a user has no way to tell _why_ a status moved;
  the mitigation is UI feedback, not making the sync less aggressive.
