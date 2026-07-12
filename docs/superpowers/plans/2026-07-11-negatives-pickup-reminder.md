# Negatives Pickup Reminder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remind the user to collect lab-developed negatives before the lab's retention window (default 30 days) expires, via a dashboard banner, a dedicated section, per-roll badges, and roll-detail controls.

**Architecture:** Additive-only. Three new columns (`labs.negative_retention_days`, `development_labs.date_negatives_picked_up`, `development_labs.negatives_not_collecting`) carry the raw facts; the pickup **state** (na / awaiting / overdue / picked-up / waived) is *derived*, never stored. The existing joined roll-list query (`RollWithDetails`) is extended to carry the ingredients + a SQL-computed deadline, so the dashboard's existing `listRolls()` feeds every read surface with no new endpoint. Pickup/waive ride the existing `PUT /api/development/lab/:id`. Nothing in the roll status machine changes.

**Tech Stack:** axum 0.8 + SeaORM 1.1 + SQLite (backend), SvelteKit + Svelte 5 runes + Tailwind 4 (frontend), vitest (frontend unit) + `cargo test` in-memory SQLite (backend integration).

**Spec:** `docs/superpowers/specs/2026-07-10-negatives-pickup-reminder-design.md`

## Global Constraints

- Rust edition 2024, workspace MSRV 1.85.0; clippy is a hard gate (`-D warnings`).
- Use the wrapper extractors `crate::extract::{Json, Path}` on handlers, never `axum::extract::*`.
- `date_received` means *"the lab notified me the order is ready"* (call/text for develop-only, email/Dropbox for develop+scan) — the countdown start — **not** physical possession. Physical possession is `date_negatives_picked_up`.
- Retention default is **30**, encoded once as `COALESCE(..., 30)` in SQL — do not backfill the column.
- Pickup/waive edits must **not** touch `date_received`, so the existing lab-dev status auto-sync stays dormant (negatives are orthogonal to the status machine).
- Frontend: never inline status pills — use a component. Svelte 5 runes only (`$state`/`$derived`/`$props`/`$bindable`). `onclick={}` on buttons.
- Run `just fmt` before every commit; `git checkout -- frontend/build/.gitkeep` if a build wiped it. Run `just check` before opening the PR.
- Bead: create one issue for this feature before Task 1 and claim it; do NOT bundle `.beads/` churn into the feature commits.

## File Structure

**Backend (create):**
- `migration/src/m20260711_000026_add_negatives_pickup.rs` — the three columns.
- `tests/negatives.rs` — integration tests for the API + SQL surface.

**Backend (modify):**
- `migration/src/lib.rs` — register migration 026.
- `entity/src/lab.rs` — add `negative_retention_days`.
- `entity/src/development_lab.rs` — add `date_negatives_picked_up`, `negatives_not_collecting`.
- `entity/src/roll_event.rs` — add `NegativesPickedUp`, `NegativesWaived` variants.
- `src/routes/labs.rs` — retention in Create/Update DTOs + handlers.
- `src/routes/development.rs` — pickup/waive in `UpdateLabDevDto` + handler + journal events.
- `src/services/roll_service.rs` — extend `RollWithDetails` struct + `ROLLS_WITH_DETAILS_SQL`.

**Frontend (create):**
- `frontend/src/lib/utils/negatives.ts` — pure derivation (`negativesState`, `isNegativesPending`).
- `frontend/src/lib/utils/negatives.test.ts` — vitest.
- `frontend/src/lib/components/ui/NegativesBadge.svelte` — the countdown pill.

**Frontend (modify):**
- `frontend/src/lib/types/index.ts` — extend `RollWithDetails`, `DevelopmentLab`, `Lab`, `RollEventType`.
- `frontend/src/lib/components/rolls/DevelopmentSection.svelte` — pickup/waive controls + state display on the lab card.
- `frontend/src/routes/(app)/rolls/[id]/+page.svelte` — pass `negativesDeadline` down.
- `frontend/src/routes/(app)/rolls/+page.svelte` — badge on roll rows.
- `frontend/src/routes/(app)/+page.svelte` — dashboard banner + "Negatives to Collect" section.
- `frontend/src/routes/(app)/labs/+page.svelte` — retention input in both dialogs.

---

### Task 1: Schema — migration + entity fields

**Files:**
- Create: `migration/src/m20260711_000026_add_negatives_pickup.rs`
- Modify: `migration/src/lib.rs`, `entity/src/lab.rs:6-14`, `entity/src/development_lab.rs:6-16`
- Test: `tests/negatives.rs`

**Interfaces:**
- Produces: `lab::Model.negative_retention_days: Option<i32>`; `development_lab::Model.date_negatives_picked_up: Option<String>`, `development_lab::Model.negatives_not_collecting: bool`.

- [ ] **Step 1: Write the failing test**

Create `tests/negatives.rs`:

```rust
mod common;

use common::open_app_with_db;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

// Proves the three new columns exist and round-trip through the entities.
#[tokio::test]
async fn new_negatives_columns_round_trip() {
    let (_app, db) = open_app_with_db().await;

    let lab = entity::lab::ActiveModel {
        name: Set("The Darkroom".into()),
        negative_retention_days: Set(Some(45)),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    assert_eq!(lab.negative_retention_days, Some(45));

    // A roll to hang the lab dev off (FK). Insert minimally via entity.
    let roll = entity::roll::ActiveModel {
        roll_id: Set("R-NEG-1".into()),
        status: Set(entity::roll::RollStatus::Shot),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let dev = entity::development_lab::ActiveModel {
        roll_id: Set(roll.id),
        lab_id: Set(Some(lab.id)),
        date_received: Set(Some("2026-07-01".into())),
        date_negatives_picked_up: Set(Some("2026-07-05".into())),
        negatives_not_collecting: Set(true),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let fetched = entity::development_lab::Entity::find_by_id(dev.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.date_negatives_picked_up.as_deref(), Some("2026-07-05"));
    assert!(fetched.negatives_not_collecting);

    // Default when unset is false.
    let dev2 = entity::development_lab::ActiveModel {
        roll_id: Set(roll.id),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    assert!(!dev2.negatives_not_collecting);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kammerz --test negatives`
Expected: FAIL — compile error, `negative_retention_days` / `date_negatives_picked_up` / `negatives_not_collecting` are not fields.

- [ ] **Step 3: Add the entity fields**

In `entity/src/lab.rs`, inside `Model`, after `pub notes: Option<String>,`:

```rust
    pub negative_retention_days: Option<i32>,
```

In `entity/src/development_lab.rs`, inside `Model`, after `pub notes: Option<String>,`:

```rust
    pub date_negatives_picked_up: Option<String>,
    pub negatives_not_collecting: bool,
```

- [ ] **Step 4: Write the migration**

Create `migration/src/m20260711_000026_add_negatives_pickup.rs`:

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Per-lab negative retention policy (days). NULL → app treats as the
        // default 30 (encoded as COALESCE in the roll-list query); no backfill.
        manager
            .alter_table(
                Table::alter()
                    .table(Labs::Table)
                    .add_column(ColumnDef::new(Labs::NegativeRetentionDays).integer().null())
                    .to_owned(),
            )
            .await?;

        // Physical-possession date (distinct from date_received, which is the
        // lab's "order ready" notification).
        manager
            .alter_table(
                Table::alter()
                    .table(DevelopmentLabs::Table)
                    .add_column(
                        ColumnDef::new(DevelopmentLabs::DateNegativesPickedUp)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // "Not collecting" opt-out. NOT NULL DEFAULT 0 so existing rows read false.
        manager
            .alter_table(
                Table::alter()
                    .table(DevelopmentLabs::Table)
                    .add_column(
                        ColumnDef::new(DevelopmentLabs::NegativesNotCollecting)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DevelopmentLabs::Table)
                    .drop_column(DevelopmentLabs::NegativesNotCollecting)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(DevelopmentLabs::Table)
                    .drop_column(DevelopmentLabs::DateNegativesPickedUp)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Labs::Table)
                    .drop_column(Labs::NegativeRetentionDays)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Labs {
    Table,
    NegativeRetentionDays,
}

#[derive(Iden)]
enum DevelopmentLabs {
    Table,
    DateNegativesPickedUp,
    NegativesNotCollecting,
}
```

- [ ] **Step 5: Register the migration**

In `migration/src/lib.rs`, add the `mod` line after the `m20260701_000025_normalize_aperture_bare;` line:

```rust
mod m20260711_000026_add_negatives_pickup;
```

and add to the `vec![...]` after the `m20260701_000025_normalize_aperture_bare::Migration` entry:

```rust
            Box::new(m20260711_000026_add_negatives_pickup::Migration),
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p kammerz --test negatives`
Expected: PASS.

- [ ] **Step 7: Run the full backend suite (migrations run in every test's setup)**

Run: `cargo test -p kammerz`
Expected: PASS — no existing test regressed by the schema change.

- [ ] **Step 8: Commit**

```bash
just fmt
git add migration/ entity/ tests/negatives.rs
git commit -m "feat(negatives): schema for lab-negative pickup tracking

Add labs.negative_retention_days, development_labs.date_negatives_picked_up
and negatives_not_collecting (migration 026) + entity fields.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Labs API — retention field

**Files:**
- Modify: `src/routes/labs.rs`
- Test: `tests/negatives.rs`

**Interfaces:**
- Consumes: `lab::Model.negative_retention_days` (Task 1).
- Produces: `POST /api/labs` accepts `negative_retention_days: number|null`; `PUT /api/labs/:id` accepts it as a nullable patch; both validate non-negative.

- [ ] **Step 1: Write the failing test** (append to `tests/negatives.rs`)

```rust
use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json, put_json};
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
async fn lab_retention_create_update_and_validation() {
    let app = open_app().await;

    // Create with retention.
    let res = app
        .clone()
        .oneshot(post_json("/api/labs", &json!({ "name": "Lab A", "negative_retention_days": 45 })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let id: i32 = json_body(res).await;

    let res = app.clone().oneshot(get(&format!("/api/labs/{id}"))).await.unwrap();
    let lab: Value = json_body(res).await;
    assert_eq!(lab["negative_retention_days"], 45);

    // Update to a new value.
    let res = app
        .clone()
        .oneshot(put_json(&format!("/api/labs/{id}"), &json!({ "negative_retention_days": 14 })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let res = app.clone().oneshot(get(&format!("/api/labs/{id}"))).await.unwrap();
    let lab: Value = json_body(res).await;
    assert_eq!(lab["negative_retention_days"], 14);

    // Negative value rejected.
    let res = app
        .clone()
        .oneshot(post_json("/api/labs", &json!({ "name": "Lab B", "negative_retention_days": -1 })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kammerz --test negatives lab_retention_create_update_and_validation`
Expected: FAIL — `negative_retention_days` is unknown to the DTOs, so it's ignored → GET returns `null`, assertion `== 45` fails.

- [ ] **Step 3: Extend the DTOs and handlers**

In `src/routes/labs.rs`:

Add `use crate::validate::validate_non_negative_i32;` alongside the existing `use crate::validate::require_nonempty;` (combine into one `use` line).

In `CreateLabDto`, after `pub notes: Option<String>,`:

```rust
    pub negative_retention_days: Option<i32>,
```

In `UpdateLabDto`, after the `notes` field:

```rust
    #[serde(deserialize_with = "double_option")]
    pub negative_retention_days: Option<Option<i32>>,
```

In `create`, after `let name = require_nonempty("name", &data.name)?;`:

```rust
    validate_non_negative_i32("negative_retention_days", data.negative_retention_days)?;
```

and in the `lab::ActiveModel { ... }` literal, after `notes: trim_opt(data.notes),`:

```rust
        negative_retention_days: Set(data.negative_retention_days),
```

In `update`, after the `if let Some(v) = data.notes { ... }` block:

```rust
    if let Some(v) = data.negative_retention_days {
        validate_non_negative_i32("negative_retention_days", v)?;
        model.negative_retention_days = Set(v);
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p kammerz --test negatives lab_retention_create_update_and_validation`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
just fmt
git add src/routes/labs.rs tests/negatives.rs
git commit -m "feat(negatives): labs API accepts negative_retention_days

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Lab dev API — pickup / waive + journal events

**Files:**
- Modify: `entity/src/roll_event.rs`, `src/routes/development.rs`
- Test: `tests/negatives.rs`

**Interfaces:**
- Consumes: `development_lab::Model.date_negatives_picked_up`, `.negatives_not_collecting` (Task 1); `RollEventService::record` (unchanged).
- Produces: `PUT /api/development/lab/:id` accepts `date_negatives_picked_up: string|null` and `negatives_not_collecting: bool`; logs `negatives_picked_up` / `negatives_waived` roll events; does NOT alter roll status.

- [ ] **Step 1: Write the failing test** (append to `tests/negatives.rs`)

```rust
// Helper: create a lab-developed roll at `lab-done` (date_received set), return
// (roll_pk, lab_dev_id). Mirrors the create flow the UI uses.
async fn lab_developed_roll(app: &axum::Router) -> (i32, i32) {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/rolls",
            &json!({ "roll_id": "R-NEG-A", "camera_id": camera_id, "status": "shot", "date_loaded": "2026-06-01" }),
        ))
        .await
        .unwrap();
    let roll_pk: i32 = json_body(res).await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "date_received": "2026-07-01" }),
        ))
        .await
        .unwrap();
    let lab_dev_id: i32 = json_body(res).await;
    (roll_pk, lab_dev_id)
}

#[tokio::test]
async fn mark_picked_up_sets_date_logs_event_and_keeps_status() {
    let app = open_app().await;
    let (roll_pk, lab_dev_id) = lab_developed_roll(&app).await;

    let status_before = {
        let res = app.clone().oneshot(get(&format!("/api/rolls/{roll_pk}"))).await.unwrap();
        let r: Value = json_body(res).await;
        r["status"].as_str().unwrap().to_string()
    };

    let res = app
        .clone()
        .oneshot(put_json(&format!("/api/development/lab/{lab_dev_id}"), &json!({ "date_negatives_picked_up": "2026-07-10" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app.clone().oneshot(get(&format!("/api/rolls/{roll_pk}/detail"))).await.unwrap();
    let detail: Value = json_body(res).await;
    assert_eq!(detail["lab_dev"]["date_negatives_picked_up"], "2026-07-10");
    // Status untouched by a pickup edit.
    assert_eq!(detail["roll"]["status"], status_before);
    // Journal recorded the specialized event.
    let types: Vec<&str> = detail["events"].as_array().unwrap().iter().map(|e| e["event_type"].as_str().unwrap()).collect();
    assert!(types.contains(&"negatives_picked_up"), "events: {types:?}");
}

#[tokio::test]
async fn mark_not_collecting_logs_waived_event() {
    let app = open_app().await;
    let (roll_pk, lab_dev_id) = lab_developed_roll(&app).await;

    let res = app
        .clone()
        .oneshot(put_json(&format!("/api/development/lab/{lab_dev_id}"), &json!({ "negatives_not_collecting": true })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app.clone().oneshot(get(&format!("/api/rolls/{roll_pk}/detail"))).await.unwrap();
    let detail: Value = json_body(res).await;
    assert_eq!(detail["lab_dev"]["negatives_not_collecting"], true);
    let types: Vec<&str> = detail["events"].as_array().unwrap().iter().map(|e| e["event_type"].as_str().unwrap()).collect();
    assert!(types.contains(&"negatives_waived"), "events: {types:?}");
}

#[tokio::test]
async fn invalid_picked_up_date_is_rejected() {
    let app = open_app().await;
    let (_roll_pk, lab_dev_id) = lab_developed_roll(&app).await;
    let res = app
        .clone()
        .oneshot(put_json(&format!("/api/development/lab/{lab_dev_id}"), &json!({ "date_negatives_picked_up": "not-a-date" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kammerz --test negatives mark_picked_up_sets_date_logs_event_and_keeps_status`
Expected: FAIL — DTO ignores the field, so `date_negatives_picked_up` stays null and the event is missing.

- [ ] **Step 3: Add the roll-event variants**

In `entity/src/roll_event.rs`, inside `RollEventType`, after the `SelfDevRemoved` variant:

```rust
    #[sea_orm(string_value = "negatives_picked_up")]
    #[serde(rename = "negatives_picked_up")]
    NegativesPickedUp,
    #[sea_orm(string_value = "negatives_waived")]
    #[serde(rename = "negatives_waived")]
    NegativesWaived,
```

- [ ] **Step 4: Extend `UpdateLabDevDto` and the handler**

In `src/routes/development.rs`, in `UpdateLabDevDto`, after the `notes` field:

```rust
    #[serde(deserialize_with = "double_option")]
    pub date_negatives_picked_up: Option<Option<String>>,
    pub negatives_not_collecting: Option<bool>,
```

In `update_lab_dev`, after the existing `if let Some(v) = data.date_received { validate_date_opt(...) }` block, add validation:

```rust
    if let Some(v) = &data.date_negatives_picked_up {
        validate_date_opt("date_negatives_picked_up", v)?;
    }
```

Immediately after `let old_received_present = existing.date_received.is_some();`, capture the pre-edit negatives state (needed to pick the right journal event and read before `existing` is moved):

```rust
    // Which specialized negatives action (if any) this edit performs — captured
    // before `existing` is consumed. Pickup takes priority over waive over a
    // plain edit for the journal entry.
    let picking_up = matches!(&data.date_negatives_picked_up, Some(Some(s)) if !s.trim().is_empty())
        && existing.date_negatives_picked_up.is_none();
    let waiving = data.negatives_not_collecting == Some(true) && !existing.negatives_not_collecting;
```

Inside the transaction closure, after the `if let Some(v) = data.notes { ... }` block, apply the new fields:

```rust
            if let Some(v) = data.date_negatives_picked_up {
                model.date_negatives_picked_up = trim_opt(v);
            }
            if let Some(v) = data.negatives_not_collecting {
                model.negatives_not_collecting = Set(v);
            }
```

Replace the existing unconditional `RollEventService::record(... LabDevEdited ...)` call with a branch that logs the salient event (pickup/waive) instead of a generic edit:

```rust
            let (event_type, summary) = if picking_up {
                (
                    entity::roll_event::RollEventType::NegativesPickedUp,
                    "Negatives picked up".to_string(),
                )
            } else if waiving {
                (
                    entity::roll_event::RollEventType::NegativesWaived,
                    "Negatives marked not for collection".to_string(),
                )
            } else {
                (
                    entity::roll_event::RollEventType::LabDevEdited,
                    "Lab development edited".to_string(),
                )
            };
            RollEventService::record(
                txn,
                result.roll_id,
                event_type,
                None,
                None,
                Some(entity::roll_event::RefKind::LabDev),
                Some(id),
                summary,
            )
            .await?;
```

(The existing `resync_lab_dev_status` block above this is unchanged — it fires only on `date_received` presence change, which pickup/waive never touches.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p kammerz --test negatives`
Expected: PASS (all negatives tests, including Tasks 1–2).

- [ ] **Step 6: Run the full backend suite (roll_events tests exercise the enum)**

Run: `cargo test -p kammerz`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
just fmt
git add entity/src/roll_event.rs src/routes/development.rs tests/negatives.rs
git commit -m "feat(negatives): pickup/waive on lab dev PUT + journal events

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Roll list query carries the pickup ingredients

**Files:**
- Modify: `src/services/roll_service.rs:20-70` (`RollWithDetails` struct + `ROLLS_WITH_DETAILS_SQL`)
- Test: `tests/negatives.rs`

**Interfaces:**
- Consumes: the schema from Task 1, the create flow from Task 3's `lab_developed_roll` helper.
- Produces: every `RollWithDetails` row (used by `GET /api/rolls`, `/for-camera`, `/{id}`, `/{id}/detail`) now carries `lab_dev_id: Option<i32>`, `lab_name: Option<String>`, `negatives_date_received: Option<String>`, `negatives_deadline: Option<String>`, `date_negatives_picked_up: Option<String>`, `negatives_not_collecting: Option<bool>`.

- [ ] **Step 1: Write the failing test** (append to `tests/negatives.rs`)

```rust
#[tokio::test]
async fn roll_list_computes_negatives_deadline_from_retention() {
    let app = open_app().await;

    // Lab with a 10-day retention.
    let res = app
        .clone()
        .oneshot(post_json("/api/labs", &json!({ "name": "Lab R", "negative_retention_days": 10 })))
        .await
        .unwrap();
    let lab_id: i32 = json_body(res).await;

    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &json!({ "roll_id": "R-DL", "camera_id": camera_id, "status": "shot" })))
        .await
        .unwrap();
    let roll_pk: i32 = json_body(res).await;
    let res = app
        .clone()
        .oneshot(post_json("/api/development/lab", &json!({ "roll_id": roll_pk, "lab_id": lab_id, "date_received": "2026-07-01" })))
        .await
        .unwrap();
    let _lab_dev_id: i32 = json_body(res).await;

    let res = app.clone().oneshot(get(&format!("/api/rolls/{roll_pk}"))).await.unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(roll["lab_name"], "Lab R");
    assert_eq!(roll["negatives_date_received"], "2026-07-01");
    assert_eq!(roll["negatives_deadline"], "2026-07-11"); // +10 days
    assert_eq!(roll["negatives_not_collecting"], false);
}

#[tokio::test]
async fn roll_without_lab_dev_has_null_negatives() {
    let app = open_app().await;
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &json!({ "roll_id": "R-NONE", "camera_id": camera_id, "status": "loaded" })))
        .await
        .unwrap();
    let roll_pk: i32 = json_body(res).await;
    let res = app.clone().oneshot(get(&format!("/api/rolls/{roll_pk}"))).await.unwrap();
    let roll: Value = json_body(res).await;
    assert!(roll["negatives_deadline"].is_null());
    assert!(roll["lab_dev_id"].is_null());
}

#[tokio::test]
async fn roll_deadline_uses_default_30_when_lab_retention_null() {
    let app = open_app().await;
    let (roll_pk, _lab_dev_id) = lab_developed_roll(&app).await; // no lab_id → retention NULL → 30
    let res = app.clone().oneshot(get(&format!("/api/rolls/{roll_pk}"))).await.unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(roll["negatives_deadline"], "2026-07-31"); // 2026-07-01 + 30
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p kammerz --test negatives roll_list_computes_negatives_deadline_from_retention`
Expected: FAIL — the fields don't exist in the JSON (deserialization of `Value` succeeds but the keys are absent → assertion fails).

- [ ] **Step 3: Extend the struct**

In `src/services/roll_service.rs`, in `RollWithDetails`, after `pub shot_count: i64,`:

```rust
    // Negatives-pickup ingredients (LEFT JOINed from the roll's lab dev + its lab).
    // All Option because a roll may have no lab dev. `negatives_deadline` is
    // date_received + retention (default 30), computed in SQL; the frontend
    // derives the live countdown/state from it.
    pub lab_dev_id: Option<i32>,
    pub lab_name: Option<String>,
    pub negatives_date_received: Option<String>,
    pub negatives_deadline: Option<String>,
    pub date_negatives_picked_up: Option<String>,
    pub negatives_not_collecting: Option<bool>,
```

- [ ] **Step 4: Extend the SQL**

In `ROLLS_WITH_DETAILS_SQL`, change the final `shot_count` select line to add the new columns after it, and add the two joins. Replace:

```rust
           (SELECT COUNT(*) FROM shots s WHERE s.roll_id = r.id) AS shot_count \
    FROM rolls r \
    LEFT JOIN cameras c ON r.camera_id = c.id \
    LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id \
    LEFT JOIN lenses l ON r.lens_id = l.id";
```

with:

```rust
           (SELECT COUNT(*) FROM shots s WHERE s.roll_id = r.id) AS shot_count, \
           dl.id AS lab_dev_id, \
           lab.name AS lab_name, \
           dl.date_received AS negatives_date_received, \
           CASE WHEN dl.date_received IS NOT NULL \
                THEN date(dl.date_received, '+' || COALESCE(lab.negative_retention_days, 30) || ' days') \
                ELSE NULL END AS negatives_deadline, \
           dl.date_negatives_picked_up AS date_negatives_picked_up, \
           dl.negatives_not_collecting AS negatives_not_collecting \
    FROM rolls r \
    LEFT JOIN cameras c ON r.camera_id = c.id \
    LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id \
    LEFT JOIN lenses l ON r.lens_id = l.id \
    LEFT JOIN development_labs dl ON dl.roll_id = r.id \
    LEFT JOIN labs lab ON dl.lab_id = lab.id";
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p kammerz --test negatives`
Expected: PASS.

- [ ] **Step 6: Run the full backend suite (rolls.rs asserts on the roll payload shape)**

Run: `cargo test -p kammerz`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
just fmt
git add src/services/roll_service.rs tests/negatives.rs
git commit -m "feat(negatives): roll list carries pickup deadline + fields

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Frontend types + shared derivation util

**Files:**
- Create: `frontend/src/lib/utils/negatives.ts`, `frontend/src/lib/utils/negatives.test.ts`
- Modify: `frontend/src/lib/types/index.ts`

**Interfaces:**
- Consumes: the backend field names from Task 4 (`negatives_date_received`, `negatives_deadline`, `date_negatives_picked_up`, `negatives_not_collecting`).
- Produces: `negativesState(input: NegativesInput, today: Date): NegativesView`; `isNegativesPending(v: NegativesView): boolean`; types `NegativesStatus`, `NegativesTier`, `NegativesView`, `NegativesInput`.

- [ ] **Step 1: Extend the TypeScript types**

In `frontend/src/lib/types/index.ts`:

In `interface RollWithDetails`, after `shot_count: number;`:

```ts
	lab_dev_id: number | null;
	lab_name: string | null;
	negatives_date_received: string | null;
	negatives_deadline: string | null;
	date_negatives_picked_up: string | null;
	negatives_not_collecting: boolean | null;
```

In `interface DevelopmentLab`, after `notes: string | null;`:

```ts
	date_negatives_picked_up: string | null;
	negatives_not_collecting: boolean;
```

In `interface Lab`, after `notes: string | null;`:

```ts
	negative_retention_days: number | null;
```

In the `RollEventType` union (starts at line 172), add two members:

```ts
	| 'negatives_picked_up'
	| 'negatives_waived'
```

- [ ] **Step 2: Write the failing test**

Create `frontend/src/lib/utils/negatives.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { isNegativesPending, negativesState, type NegativesInput } from './negatives';

const base: NegativesInput = {
	negatives_date_received: '2026-07-01',
	negatives_deadline: '2026-07-31',
	date_negatives_picked_up: null,
	negatives_not_collecting: null
};
const on = (d: string) => new Date(`${d}T12:00:00`);

describe('negativesState', () => {
	it('is na when the order is not complete (no received date)', () => {
		const v = negativesState({ ...base, negatives_date_received: null, negatives_deadline: null }, on('2026-07-10'));
		expect(v.status).toBe('na');
		expect(v.tier).toBe('none');
		expect(v.label).toBe('');
	});

	it('is awaiting/far more than 7 days out', () => {
		const v = negativesState(base, on('2026-07-10')); // 21 days left
		expect(v.status).toBe('awaiting');
		expect(v.daysLeft).toBe(21);
		expect(v.tier).toBe('far');
		expect(v.label).toBe('21d left');
	});

	it('is near within 7 days', () => {
		const v = negativesState(base, on('2026-07-25')); // 6 days
		expect(v.tier).toBe('near');
	});

	it('is soon within 3 days', () => {
		const v = negativesState(base, on('2026-07-29')); // 2 days
		expect(v.tier).toBe('soon');
	});

	it('labels the deadline day itself as due today (still awaiting)', () => {
		const v = negativesState(base, on('2026-07-31'));
		expect(v.status).toBe('awaiting');
		expect(v.daysLeft).toBe(0);
		expect(v.label).toBe('Due today');
	});

	it('is overdue past the deadline', () => {
		const v = negativesState(base, on('2026-08-05')); // -5
		expect(v.status).toBe('overdue');
		expect(v.daysLeft).toBe(-5);
		expect(v.tier).toBe('overdue');
		expect(v.label).toBe('OVERDUE');
	});

	it('is picked-up when a pickup date is set', () => {
		const v = negativesState({ ...base, date_negatives_picked_up: '2026-07-05' }, on('2026-08-05'));
		expect(v.status).toBe('picked-up');
		expect(v.label).toBe('Collected');
		expect(v.tier).toBe('none');
	});

	it('is waived (takes priority over a pickup date)', () => {
		const v = negativesState({ ...base, negatives_not_collecting: true, date_negatives_picked_up: '2026-07-05' }, on('2026-08-05'));
		expect(v.status).toBe('waived');
		expect(v.label).toBe('Not collecting');
	});

	it('defaults null not_collecting to false', () => {
		const v = negativesState({ ...base, negatives_not_collecting: null }, on('2026-07-10'));
		expect(v.status).toBe('awaiting');
	});
});

describe('isNegativesPending', () => {
	it('is true only for awaiting and overdue', () => {
		expect(isNegativesPending(negativesState(base, on('2026-07-10')))).toBe(true); // awaiting
		expect(isNegativesPending(negativesState(base, on('2026-08-05')))).toBe(true); // overdue
		expect(isNegativesPending(negativesState({ ...base, date_negatives_picked_up: '2026-07-05' }, on('2026-08-05')))).toBe(false);
		expect(isNegativesPending(negativesState({ ...base, negatives_date_received: null, negatives_deadline: null }, on('2026-07-10')))).toBe(false);
	});
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd frontend && bun run test:unit negatives`
Expected: FAIL — `./negatives` module not found.

- [ ] **Step 4: Write the util**

Create `frontend/src/lib/utils/negatives.ts`:

```ts
// Pure derivation of a lab dev's negatives-pickup state. Single source of truth
// for the dashboard banner/section, roll-list badges, and the roll-detail card.
// `today` is injected so the countdown is deterministic in tests and live in the
// UI (callers pass `new Date()`).

export type NegativesStatus = 'na' | 'awaiting' | 'overdue' | 'picked-up' | 'waived';
export type NegativesTier = 'none' | 'far' | 'near' | 'soon' | 'overdue';

export interface NegativesInput {
	negatives_date_received: string | null;
	negatives_deadline: string | null;
	date_negatives_picked_up: string | null;
	negatives_not_collecting: boolean | null;
}

export interface NegativesView {
	status: NegativesStatus;
	/** date_received + retention, or null when the order isn't complete. */
	deadline: string | null;
	/** Whole days until the deadline (negative once overdue); null unless awaiting/overdue. */
	daysLeft: number | null;
	tier: NegativesTier;
	/** Short badge text; '' when there is nothing to show. */
	label: string;
}

function midnight(d: Date): number {
	return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

// Parse a 'YYYY-MM-DD' as a local date (avoids the UTC shift of `new Date(str)`).
function parseLocalDate(s: string): Date {
	const [y, m, d] = s.split('-').map(Number);
	return new Date(y, m - 1, d);
}

export function negativesState(input: NegativesInput, today: Date): NegativesView {
	const deadline = input.negatives_deadline;

	if (input.negatives_not_collecting) {
		return { status: 'waived', deadline, daysLeft: null, tier: 'none', label: 'Not collecting' };
	}
	if (input.date_negatives_picked_up) {
		return { status: 'picked-up', deadline, daysLeft: null, tier: 'none', label: 'Collected' };
	}
	if (!input.negatives_date_received || !deadline) {
		return { status: 'na', deadline: null, daysLeft: null, tier: 'none', label: '' };
	}

	const daysLeft = Math.round((midnight(parseLocalDate(deadline)) - midnight(today)) / 86_400_000);

	if (daysLeft < 0) {
		return { status: 'overdue', deadline, daysLeft, tier: 'overdue', label: 'OVERDUE' };
	}
	const tier: NegativesTier = daysLeft <= 3 ? 'soon' : daysLeft <= 7 ? 'near' : 'far';
	const label = daysLeft === 0 ? 'Due today' : `${daysLeft}d left`;
	return { status: 'awaiting', deadline, daysLeft, tier, label };
}

export function isNegativesPending(v: NegativesView): boolean {
	return v.status === 'awaiting' || v.status === 'overdue';
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd frontend && bun run test:unit negatives`
Expected: PASS (all cases).

- [ ] **Step 6: Type-check**

Run: `cd frontend && bun run check`
Expected: PASS (no svelte-check/tsc errors from the new types).

- [ ] **Step 7: Commit**

```bash
just fmt
git add frontend/src/lib/utils/negatives.ts frontend/src/lib/utils/negatives.test.ts frontend/src/lib/types/index.ts
git commit -m "feat(negatives): frontend types + shared derivation util

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: NegativesBadge component + roll-list badge + roll-detail controls

**Files:**
- Create: `frontend/src/lib/components/ui/NegativesBadge.svelte`
- Modify: `frontend/src/lib/components/rolls/DevelopmentSection.svelte`, `frontend/src/routes/(app)/rolls/[id]/+page.svelte`, `frontend/src/routes/(app)/rolls/+page.svelte`

**Interfaces:**
- Consumes: `negativesState`, `isNegativesPending`, `NegativesView` (Task 5); `updateLabDev` (existing); `todayLocal()` (existing in DevelopmentSection).
- Produces: `<NegativesBadge view={NegativesView} />`; a `negativesDeadline` prop on `DevelopmentSection`.

- [ ] **Step 1: Create the badge component**

Create `frontend/src/lib/components/ui/NegativesBadge.svelte`:

```svelte
<script lang="ts">
	import type { NegativesView } from '$lib/utils/negatives';

	let { view }: { view: NegativesView } = $props();

	// Tier → pill classes, using existing theme tokens (accent = amber, danger = red).
	const tierClasses: Record<NegativesView['tier'], string> = {
		none: '',
		far: 'bg-surface-overlay text-text-faint',
		near: 'bg-accent/10 text-accent',
		soon: 'bg-accent/20 text-accent',
		overdue: 'bg-danger/15 text-danger-fg'
	};
</script>

{#if view.label}
	<span
		class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium {tierClasses[view.tier]}"
		title="Negatives to collect"
	>
		{view.label}
	</span>
{/if}
```

- [ ] **Step 2: Manual visual check via the design-system pass**

Run: `cd frontend && bun run check`
Expected: PASS. (Exact colors may be refined later by the `design-system` skill; these use real tokens and are valid now.)

- [ ] **Step 3: Add pickup/waive controls to the lab card**

In `frontend/src/lib/components/rolls/DevelopmentSection.svelte`:

Add imports at the top of the `<script>` (near the other UI imports):

```ts
	import NegativesBadge from '$lib/components/ui/NegativesBadge.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import { negativesState } from '$lib/utils/negatives';
```

Add a `negativesDeadline` prop. In the `let { ... } = $props()` destructure add `negativesDeadline = null,` and in its `Props` type add:

```ts
		/** roll.negatives_deadline (date_received + retention), for the pickup countdown. */
		negativesDeadline?: string | null;
```

Add derived state + a waive-confirm flag after the existing `$state`/`$derived` declarations:

```ts
	let showWaiveConfirm = $state(false);

	const negView = $derived(
		labDev
			? negativesState(
					{
						negatives_date_received: labDev.date_received,
						negatives_deadline: negativesDeadline,
						date_negatives_picked_up: labDev.date_negatives_picked_up,
						negatives_not_collecting: labDev.negatives_not_collecting
					},
					new Date()
				)
			: null
	);

	async function markPickedUp() {
		if (!labDev) return;
		await updateLabDev(labDev.id, { date_negatives_picked_up: todayLocal() });
		onchange();
	}

	async function markNotCollecting() {
		if (!labDev) return;
		await updateLabDev(labDev.id, { negatives_not_collecting: true });
		showWaiveConfirm = false;
		onchange();
	}
```

In the lab-card markup (the `{#if labDev}` block, after the `{#if labDev.date_received}...{/if}` line ~396), add the negatives row:

```svelte
				{#if negView && negView.status !== 'na'}
					<div class="mt-2 flex flex-wrap items-center gap-2 border-t border-border-subtle pt-2">
						<span class="text-xs font-semibold uppercase tracking-wider text-text-faint">Negatives</span>
						{#if negView.status === 'picked-up'}
							<span class="text-text-muted">Collected · {labDev.date_negatives_picked_up}</span>
						{:else if negView.status === 'waived'}
							<span class="text-text-muted">Not collecting</span>
						{:else}
							<NegativesBadge view={negView} />
							<Button size="sm" variant="ghost" onclick={markPickedUp}>Mark picked up</Button>
							<Button size="sm" variant="ghost" onclick={() => (showWaiveConfirm = true)}>Not collecting</Button>
						{/if}
					</div>
				{/if}
```

At the end of the component markup (after the existing dialogs), add the confirm dialog:

```svelte
<ConfirmDialog
	bind:open={showWaiveConfirm}
	title="Not collecting negatives?"
	message="This silences the pickup reminder for this roll. You can still see the lab dev record."
	confirmLabel="Not collecting"
	onconfirm={markNotCollecting}
/>
```

(If `ConfirmDialog`'s prop names differ, match the existing usages already in this file / the roll detail page — `title`, `message`, `confirmLabel`, `onconfirm`, `bind:open`.)

- [ ] **Step 4: Pass the deadline from the roll detail page**

In `frontend/src/routes/(app)/rolls/[id]/+page.svelte`, in the `<DevelopmentSection ... />` usage (~line 1186), add the prop:

```svelte
				negativesDeadline={roll?.negatives_deadline ?? null}
```

- [ ] **Step 5: Add the badge to the roll list**

In `frontend/src/routes/(app)/rolls/+page.svelte`, add the imports:

```ts
	import NegativesBadge from '$lib/components/ui/NegativesBadge.svelte';
	import { negativesState, isNegativesPending } from '$lib/utils/negatives';
```

In the roll-row markup, next to the roll's status `<Badge>`, render the countdown when pending:

```svelte
					{#if isNegativesPending(negativesState(roll, new Date()))}
						<NegativesBadge view={negativesState(roll, new Date())} />
					{/if}
```

(`roll` is a `RollWithDetails`, which now satisfies `NegativesInput` structurally — no adapter needed.)

- [ ] **Step 6: Type-check and run the app**

Run: `cd frontend && bun run check`
Expected: PASS.

Then drive it end-to-end (use the `verify` skill / `just dev` + browser): create a lab-dev roll with a `date_received`, confirm the roll detail shows a countdown badge + "Mark picked up" / "Not collecting", click "Mark picked up", confirm the card flips to "Collected · <today>" and the reminder disappears.

- [ ] **Step 7: Commit**

```bash
just fmt
git checkout -- frontend/build/.gitkeep 2>/dev/null || true
git add frontend/src/lib/components/ui/NegativesBadge.svelte frontend/src/lib/components/rolls/DevelopmentSection.svelte "frontend/src/routes/(app)/rolls/[id]/+page.svelte" "frontend/src/routes/(app)/rolls/+page.svelte"
git commit -m "feat(negatives): badge + pickup/waive controls on roll detail & list

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Dashboard banner + "Negatives to Collect" section

**Files:**
- Modify: `frontend/src/routes/(app)/+page.svelte`

**Interfaces:**
- Consumes: `negativesState`, `isNegativesPending` (Task 5); `NegativesBadge` (Task 6); `updateLabDev` (existing); `rolls: RollWithDetails[]` + `load()` (existing in this file).
- Produces: no exports (page component).

- [ ] **Step 1: Add derivations + imports**

In `frontend/src/routes/(app)/+page.svelte` `<script>`:

```ts
	import NegativesBadge from '$lib/components/ui/NegativesBadge.svelte';
	import { negativesState, isNegativesPending } from '$lib/utils/negatives';
	import { updateLabDev } from '$lib/api/development';
```

Add derived lists (near the other `$derived` roll groupings):

```ts
	// Rolls whose negatives are still at the lab (awaiting or overdue), each with
	// its live view. Sorted ascending by daysLeft → most-overdue first, then
	// soonest deadline (overdue has negative daysLeft).
	const negativesPending = $derived(
		rolls
			.map((roll) => ({ roll, view: negativesState(roll, new Date()) }))
			.filter((x) => isNegativesPending(x.view))
			.sort((a, b) => (a.view.daysLeft ?? 0) - (b.view.daysLeft ?? 0))
	);
	const negativesOverdueCount = $derived(negativesPending.filter((x) => x.view.status === 'overdue').length);

	async function pickUpFromDashboard(rollLabDevId: number | null) {
		if (rollLabDevId == null) return;
		await updateLabDev(rollLabDevId, { date_negatives_picked_up: new Date().toISOString().slice(0, 10) });
		await load();
	}
```

- [ ] **Step 2: Add the banner** (top of the dashboard markup, inside the outer container, before the first `<FadeIn>`):

```svelte
	{#if negativesPending.length > 0}
		<div
			class="mb-4 rounded-lg border px-4 py-3 {negativesOverdueCount > 0
				? 'border-danger/50 bg-danger/10 text-danger-fg'
				: 'border-accent/40 bg-accent/10 text-accent'}"
		>
			<a href="#negatives-to-collect" class="font-medium">
				{negativesPending.length}
				{negativesPending.length === 1 ? 'roll' : 'rolls'} of negatives to collect{negativesOverdueCount > 0
					? ` — ${negativesOverdueCount} overdue`
					: ''}.
			</a>
		</div>
	{/if}
```

- [ ] **Step 3: Add the section** (as a peer of the other sections, e.g. after "Needs Attention"):

```svelte
	{#if negativesPending.length > 0}
		<section id="negatives-to-collect">
			<FadeIn delay={250}>
				<h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-text-faint">Negatives to Collect</h2>
				<div class="divide-y divide-border-subtle">
					{#each negativesPending as { roll, view } (roll.id)}
						<div class="flex flex-wrap items-center gap-3 px-4 py-2.5">
							<a href="/rolls/{roll.id}?from=dashboard" class="font-mono text-sm text-text hover:text-accent">{roll.roll_id}</a>
							{#if roll.film_stock_brand}
								<span class="text-sm text-text-muted">{roll.film_stock_brand} {roll.film_stock_name ?? ''}</span>
							{/if}
							{#if roll.lab_name}
								<span class="text-sm text-text-faint">{roll.lab_name}</span>
							{/if}
							<NegativesBadge {view} />
							<div class="ml-auto">
								<Button size="sm" variant="ghost" onclick={() => pickUpFromDashboard(roll.lab_dev_id)}>Picked up</Button>
							</div>
						</div>
					{/each}
				</div>
			</FadeIn>
		</section>
	{/if}
```

(If `Button` isn't already imported in this file, add `import Button from '$lib/components/ui/Button.svelte';`.)

- [ ] **Step 4: Type-check + drive it**

Run: `cd frontend && bun run check`
Expected: PASS.

Then via `just dev` + browser: with at least one pending roll, confirm the banner appears (amber; red when an overdue roll exists), the "Negatives to Collect" section lists it, the banner link scrolls to the section, and the section "Picked up" button clears the roll from both banner and section on reload.

- [ ] **Step 5: Commit**

```bash
just fmt
git checkout -- frontend/build/.gitkeep 2>/dev/null || true
git add "frontend/src/routes/(app)/+page.svelte"
git commit -m "feat(negatives): dashboard banner + Negatives to Collect section

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: Labs page — retention input

**Files:**
- Modify: `frontend/src/routes/(app)/labs/+page.svelte`

**Interfaces:**
- Consumes: `createLab`/`updateLab` (existing) with the `negative_retention_days` field (Task 2 backend, Task 5 type).
- Produces: no exports (page component).

- [ ] **Step 1: Add the form state**

In `frontend/src/routes/(app)/labs/+page.svelte` `<script>`, next to the other field `$state`s (`name`, `location`, …):

```ts
	let retentionDays = $state('');
```

In `resetForm()`, add:

```ts
		retentionDays = '';
```

In `openEditDialog` (where it seeds `name`, `location`, … from `lab`), add:

```ts
		retentionDays = lab.negative_retention_days?.toString() ?? '';
```

- [ ] **Step 2: Include it in create + update payloads**

In the create handler's payload object (where `location: location || null,` etc. are set), add:

```ts
				negative_retention_days: retentionDays ? parseInt(retentionDays, 10) : null,
```

In the `updateLab(editingLab.id, { ... })` payload, add the same line.

- [ ] **Step 3: Add the input to both dialogs**

In the Add dialog (after the `<Textarea label="Notes" ... />`, ~line 178) and again in the Edit dialog (after its Notes textarea, ~line 208):

```svelte
		<Input label="Negative retention (days)" type="number" bind:value={retentionDays} placeholder="30" />
```

(If `Input` doesn't pass `type` through, it does via `{...rest}` on the native input per the component pattern; a `number`-typed input is fine.)

- [ ] **Step 4: Type-check + drive it**

Run: `cd frontend && bun run check`
Expected: PASS.

Then via `just dev` + browser: edit a lab, set retention to e.g. 14, save; create a lab-dev roll with that lab + a `date_received`, and confirm the roll's countdown reflects the 14-day window (deadline = received + 14).

- [ ] **Step 5: Commit**

```bash
just fmt
git checkout -- frontend/build/.gitkeep 2>/dev/null || true
git add "frontend/src/routes/(app)/labs/+page.svelte"
git commit -m "feat(negatives): per-lab negative retention input

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: Full gate + PR

**Files:** none (verification).

- [ ] **Step 1: Run the full local CI mirror**

Run: `just check`
Expected: PASS — fmt-check, `ci-backend` (build/clippy/test `--locked`), `ci-frontend` (bun install + svelte-check + vitest + build). Fix any clippy lint (prefer a real fix over `#[allow]`).

- [ ] **Step 2: Restore the build placeholder if wiped**

Run: `git status` — if `frontend/build/.gitkeep` shows deleted, run `git checkout -- frontend/build/.gitkeep`.

- [ ] **Step 3: Push and open the PR**

```bash
git push -u origin feat/negatives-pickup-reminder
gh pr create --title "feat: negatives pickup reminder" --body "$(cat <<'EOF'
Reminds the user to collect lab-developed negatives before the lab's retention
window (default 30 days) expires. Parallel to the roll status machine.

- Schema: labs.negative_retention_days + development_labs pickup date/waive flag (migration 026)
- Derived state (na/awaiting/overdue/picked-up/waived), no stored status
- Dashboard banner + "Negatives to Collect" section, roll-list/detail badges, pickup/waive controls
- Per-lab retention setting; journal events for pickup/waive

Spec: docs/superpowers/specs/2026-07-10-negatives-pickup-reminder-design.md
Plan: docs/superpowers/plans/2026-07-11-negatives-pickup-reminder.md

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 4: Post-merge (separate commit on `main`)** — close the bead: `bd close <id> --reason="PR #<N> merged"`, then `bd export -o .beads/issues.jsonl` if stale, commit as `chore(beads): close <id> after PR #<N> merge`, and `bd dolt push` + `git push`.

---

## Self-Review

**Spec coverage:**
- Schema (labs retention, pickup date, waive flag) → Task 1. ✓
- Derivation (na/awaiting/overdue/picked-up/waived, clock on `date_received`, deadline = received + retention default 30) → Task 4 (SQL deadline) + Task 5 (state). ✓
- No new endpoints; roll-list query extended; pickup/waive via lab-dev PUT → Task 3 + Task 4. ✓
- Labs retention DTO + validation → Task 2. ✓
- Journal events (`negatives_picked_up`, `negatives_waived`) → Task 3. ✓
- Status auto-sync untouched by pickup/waive → asserted in Task 3 (`mark_picked_up..._keeps_status`). ✓
- Urgency tiers (overdue / ≤3 / ≤7 / >7) → Task 5 (`tier`) + Task 6 (`NegativesBadge`). ✓
- Dashboard banner + section (overdue-first sort) → Task 7. ✓
- Roll list/detail badge + detail controls (pickup no-confirm, waive light confirm) → Task 6. ✓
- Lab add/edit retention input (placeholder 30) → Task 8. ✓
- Testing: backend integration (`tests/negatives.rs`) + vitest (`negatives.test.ts`) + existing smoke → Tasks 1–5, Task 9. ✓
- Out of scope (auto-expire, per-roll override, notifications, self-dev) → not implemented. ✓

**Placeholder scan:** No TBD/TODO; every code step shows real code; the two "if prop names differ, match existing usage" notes point at concrete in-repo references (ConfirmDialog usages, Input `{...rest}`) rather than deferring content.

**Type consistency:** `negativesState`/`isNegativesPending`/`NegativesView`/`NegativesInput`/`NegativesTier`/`NegativesStatus` used identically across Tasks 5–7. Backend field names (`negatives_date_received`, `negatives_deadline`, `date_negatives_picked_up`, `negatives_not_collecting`, `lab_dev_id`, `lab_name`) match between the SQL (Task 4), the TS `RollWithDetails` (Task 5), and the util input. `RollWithDetails` is used directly as `NegativesInput` (structural match confirmed: all four input fields present on the interface).
