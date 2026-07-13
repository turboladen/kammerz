# ADR-0009: Negatives-pickup as derived state, parallel to the roll status machine

- **Status:** Accepted
- **Date:** 2026-07-10
- **Related:** `frontend/src/lib/utils/negatives.ts`, `src/services/roll_service.rs`
  (`RollWithDetails.negatives_deadline` SQL), `migration/src/m20260711_000026_add_negatives_pickup.rs`,
  bead kammerz-lam (PR #119)

## Context

A lab-developed roll's physical negatives stay at the lab after the order
completes, and the lab discards them after a retention window (typically
~30 days, configurable per lab). Kammerz had no way to surface "go pick up
your negatives," so this deadline could pass silently.

Pickup only becomes relevant once the lab notifies that the order is ready —
which this app already records as the lab dev's `date_received` field — and
from that point pickup runs **parallel** to the rest of the pipeline: the
negatives can sit uncollected while the roll advances through `scanned`,
`post-processed`, or `archived`. It cannot be a new roll status because it
isn't a stage the roll passes through in sequence; it's an independent
concern that happens to share a start date with an existing field. Only
lab-developed rolls have it — self-developed rolls already hold their own
negatives.

## Decision

Model pickup as **fully derived state**, never a stored status enum:

- **Additive schema only** (`migration/src/m20260711_000026_add_negatives_pickup.rs`):
  `labs.negative_retention_days` (nullable, default-30-when-null),
  `development_labs.date_negatives_picked_up`, and
  `development_labs.negatives_not_collecting` (boolean waive). No new roll
  status, no table rebuild.
- **Deadline computed in SQL**, not cached: `RollWithDetails` in
  `roll_service.rs` derives `negatives_deadline` as
  `date(dl.date_received, '+' || COALESCE(lab.negative_retention_days, 30) || ' days')`,
  `NULL` when `date_received` is unset (order not yet complete → no pickup
  state at all).
- **Single client-side derivation function**: `negativesState()` in
  `frontend/src/lib/utils/negatives.ts` is the one place that turns
  `{negatives_date_received, negatives_deadline, date_negatives_picked_up,
  negatives_not_collecting}` + `today` into a `NegativesStatus` (`na` |
  `awaiting` | `overdue` | `picked-up` | `waived`) plus a countdown tier.
  Every surface (dashboard banner/section, list badges, roll-detail card)
  reads through this one function — no inline date math anywhere else.
- **Parallel to, never coupled with, the status machine**: pickup and waive
  writes only touch the two new fields; they never write `date_received`.
  The roll-status auto-sync (`sync_lab_dev_status` et al.) fires only on a
  change in `date_received` _presence_, so marking negatives picked up or
  waived is structurally incapable of moving the roll's status, and no
  change was needed to `status-flows.json`, `RollStatus`, or the chevron
  progression bar.

## Consequences

- **Positive:** the pickup state can never drift from the data that defines
  it — there's no enum value to fall out of sync with dates/flags. A roll's
  pickup state is always a pure function of columns that already exist for
  other reasons (`date_received`) plus the two new ones.
- **Positive:** reuses the existing `PUT /api/development/lab/:id` route for
  both "mark picked up" and "waive" — no new endpoints, no new journal
  plumbing beyond two new `RollEventType` variants.
- **Positive:** keeping it parallel means the status machine's invariants
  (forward-only auto-sync, backward-move confirmation) needed zero changes —
  the two concerns are provably independent by construction, not just by
  convention.
- **Negative / limits:** the countdown never auto-expires — an overdue roll
  keeps signaling until a human resolves it (deliberate; documented as
  out-of-scope auto-expiry, not a bug). Retention is a per-lab policy only,
  not per-roll — a roll needing a longer/shorter window than its lab's
  default has no override. Notification is on-screen only (LAN catalog, no
  push/email).
