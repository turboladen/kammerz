# Negatives Pickup Reminder — Design

**Date:** 2026-07-10
**Status:** Approved (brainstorm), pending implementation plan

## Problem

When a roll is developed at a local lab, the physical negatives stay at the lab
after the order is complete. The lab discards uncollected negatives after a
retention window (typically ~30 days). Kammerz currently has no way to surface
"go pick up your negatives," so rolls whose negatives are still at the lab can
silently pass their disposal deadline.

Pickup only becomes relevant **once the lab has completed the order** — either
develop-only or develop + scan, depending on what was requested. At that point
the lab notifies that the results are ready:

- **develop-only** → a phone call / text that the negatives are ready
- **develop + scan** → an email with a Dropbox link to the scans

Either notification is the moment the negatives (and any scans) become
available — and it is the **start of the ~30-day retention countdown**. This
maps directly to the lab dev record's existing `date_received` field, which in
this app means *"the lab told me the order is ready"* (the notification date),
**not** *"I physically have the negatives in hand."* Physical possession is the
new, separate `date_negatives_picked_up`. The develop-only vs develop+scan
distinction changes only *how* you're notified, not the model — both set
`date_received` and start the same clock, so the feature treats them identically.

Once the order is complete, pickup runs **parallel** to the rest of the pipeline:
the negatives can sit uncollected while the roll advances to `scanned`,
`post-processed`, or even `archived`, and pickup can be resolved at any time
after the order completes. It is therefore modeled as a derived concern on the
**lab development record**, not as a new roll status. Before the order is
complete (`date_received` unset), there is no pickup state at all
(`NotApplicable`).

Only **lab-developed** rolls have this problem — self-developed rolls already
hold their own negatives.

## Data Model (Approach A — additive fields, everything derived)

One migration adds three columns. No stored status enum — pickup state is fully
derivable from dates + two flags, so it can never drift.

### Schema additions

| Table               | Column                        | Type                     | Notes |
|---------------------|-------------------------------|--------------------------|-------|
| `labs`              | `negative_retention_days`     | `INTEGER NULL`           | Per-lab retention policy. `NULL` → treated as default **30**. Nullable so existing labs need no backfill; the default lives in one place. |
| `development_labs`  | `date_negatives_picked_up`    | `TEXT NULL`              | Stamped when negatives are collected. |
| `development_labs`  | `negatives_not_collecting`    | `INTEGER NOT NULL DEFAULT 0` | Boolean "waive" opt-out for rolls whose negatives you don't want back. |

Migration follows the idempotent-guard convention (`ADD COLUMN` is inherently
additive; no table rebuild, so no FK-cascade concern).

### Derived state (pure function)

Evaluated per lab dev record, given `today`:

```
if negatives_not_collecting        → Waived         (silent, resolved)
else if date_negatives_picked_up   → PickedUp       (silent, resolved)
else if date_received is None      → NotApplicable  (dev not done; clock not started)
else:
    deadline = date_received + retention_days       (retention_days = lab value or 30)
    if today > deadline            → Overdue         (escalated; keeps reminding)
    else                           → AwaitingPickup  (counting down)
```

A roll surfaces a reminder iff its lab dev is `AwaitingPickup` or `Overdue`.
The countdown never auto-expires — an overdue roll keeps shouting until the user
marks it picked up or waives it.

The **clock starts on the existing `date_received`** field — the lab's
"order ready" notification date (call/text for develop-only, email/Dropbox for
develop + scan). No new "ready" date to remember.

## API & Backend

**No new endpoints.** All behavior hangs off existing routes.

1. **Roll list query** — `RollWithDetails` in `roll_service.rs` LEFT JOINs the
   roll's lab dev and its lab, adding:
   - `lab_dev_id`, `lab_name`
   - `negatives_date_received`
   - `negatives_deadline` — computed in SQL:
     `date(dl.date_received, '+' || COALESCE(l.negative_retention_days, 30) || ' days')`,
     `NULL` when `date_received` is `NULL`
   - `date_negatives_picked_up`, `negatives_not_collecting`

   One join, no N+1. This single payload (already loaded by the dashboard's
   `listRolls()`) feeds the banner, the section, and the list badges.

2. **Lab dev create/update** — extend `CreateLabDevDto` / `UpdateLabDevDto` and
   the entity mapping with `date_negatives_picked_up` (`Option<Option<String>>`
   on update via `double_option`) and `negatives_not_collecting` (`Option<bool>`).
   Pickup and waive are plain PUTs to the existing `PUT /api/development/lab/:id`:
   - "Mark picked up" → set `date_negatives_picked_up = today`
   - "Not collecting" → set `negatives_not_collecting = true`
   Validate `date_negatives_picked_up` with `validate_date_opt`.

3. **Labs create/update DTOs** — add `negative_retention_days`
   (`Option<i32>` on create, `Option<Option<i32>>` on update), validated with
   `validate_non_negative_i32`.

4. **Roll detail composite** — `GET /api/rolls/{id}/detail` adds the three new
   lab-dev fields to its lab-dev struct so the detail card renders countdown +
   controls.

5. **Journal events** — extend `RollEventType` (total enum; forces match
   arms updated) with `negatives_picked_up` and `negatives_waived`, logged from
   the lab dev update path, matching existing `lab_dev_edited` etc.

### Invariants preserved

- The lab-dev status auto-sync fires **only on `date_received` presence change**.
  The new fields don't touch `date_received`, so editing pickup/waive/retention
  never relocates the roll's status — correct, since negatives are orthogonal to
  the status machine.
- No change to `status-flows.json`, the `RollStatus` enum, the flow arrays, or
  the chevron progression bar.

## Frontend & UX

### Shared derivation (single source of truth)

`src/lib/utils/negatives.ts` — pure `negativesState(row, today)` returning
`{ state, deadline, daysLeft }` with
`state ∈ awaiting | overdue | picked-up | waived | na`, plus `urgencyTier(daysLeft)`.
Every surface reads from this one function; no inline date math anywhere.
Vitest unit tests, matching the coverage-scoped `lib/utils` pattern.

Extend `RollWithDetails` and the lab-dev types in `src/lib/types/index.ts` with
the new fields.

### Urgency tiers

Exact palette chosen via the design-system skill at implementation time:

| Condition        | Treatment            |
|------------------|----------------------|
| `overdue`        | red / alert, loudest |
| `daysLeft ≤ 3`   | strong amber         |
| `daysLeft ≤ 7`   | amber                |
| `daysLeft > 7`   | neutral / muted      |

### Surfaces

1. **Dashboard banner** — new component above the sections, rendered only when
   any roll is `awaiting`/`overdue`, e.g. *"3 rolls of negatives to collect — 1
   overdue."* Red accent if any overdue. Click scrolls to the section.

2. **Dashboard section "Negatives to Collect"** — peer of *In the Field* /
   *Needs Attention*. List rows sorted **overdue-first (most overdue first),
   then soonest deadline**. Each row: roll id + film, lab name, countdown
   `<Badge>`, one-click **"Picked up"** action (stamps today). Uses the existing
   list-row + ledger/`GroupHeader` conventions.

3. **Roll list/detail badge** — countdown `<Badge>` (`12d left` / `OVERDUE`) on
   roll rows in list views. Always the `<Badge>` component, never inline pills.

4. **Roll detail — lab dev card** — countdown badge + controls:
   - **"Mark picked up"** → PUT date = today (non-destructive, no confirm).
     Resolved card shows *"Negatives collected · <date>"*.
   - **"Not collecting"** → flips the waive bool behind a light `ConfirmDialog`
     (it silences a real deadline). Resolved card shows *"Not collecting
     negatives."*
   - Both surface the transient `InlineNotice` on reload, consistent with the
     existing auto-sync feedback.

5. **Lab add/edit dialog** — a **"Negative retention (days)"** number input,
   placeholder `30`, empty → default 30.

## Testing

- **Backend:** integration tests for the extended lab-dev update (pickup/waive
  set the fields; journal events written), labs retention field round-trip, and
  the `RollWithDetails` deadline SQL (verify `negatives_deadline` computes
  `date_received + retention` and is `NULL` without `date_received`). Assert the
  auto-sync is NOT triggered by pickup/waive edits.
- **Frontend:** vitest for `negativesState` / `urgencyTier` across all five
  states and tier boundaries (na, awaiting >7 / ≤7 / ≤3, overdue, picked-up,
  waived), including the retention default (`NULL` → 30).
- **e2e:** covered by the existing smoke gate; no new Playwright spec required
  unless a surface proves flaky.

## Out of Scope (YAGNI)

- Auto-expire / "likely disposed" state (deliberately rejected — overdue keeps
  shouting until resolved).
- Per-roll retention override (retention is a lab policy).
- Push/email notifications — the app is a LAN catalog; on-screen surfacing is
  the mechanism.
- Self-developed rolls (no lab-held negatives).
