# Roll Activity Log (Phase 1 — Backend) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an append-only `roll_events` activity log to the backend, emit events on every roll/shot/development mutation, and expose them on the roll detail endpoint — the data foundation for the redesigned roll page (Phase 2).

**Architecture:** A new `roll_events` table + SeaORM entity. A thin `RollEventService` does all inserts/reads. Status-change events are emitted inside the two functions every status write funnels through (`RollService::auto_sync_status` and `advance_status_along`) plus the manual `PUT /api/rolls/{id}` handler; shot and development events are emitted in their route handlers (inside the existing transactions). `events` is added to the composite `GET /api/rolls/{id}/detail` so the page still loads in one round-trip. No backfill — the app has never been deployed.

**Tech Stack:** Rust, axum 0.8, SeaORM 1.1, SQLite. Integration tests drive the real router over HTTP (`tests/common`).

**Spec:** `docs/superpowers/specs/2026-06-13-roll-detail-redesign-design.md`

---

## File structure

- `entity/src/roll_event.rs` — **new** entity (Model + `RollEventType` + `RefKind` enums + Relation to roll).
- `entity/src/lib.rs` — **modify** register `pub mod roll_event;`.
- `migration/src/m20260614_000022_create_roll_events.rs` — **new** create-table migration.
- `migration/src/lib.rs` — **modify** register the migration.
- `src/services/roll_event_service.rs` — **new** `RollEventService` (`record`, `record_status_change`, `list_for_roll`).
- `src/services/mod.rs` — **modify** register `pub mod roll_event_service;`.
- `src/services/roll_service.rs` — **modify** emit `status_changed` inside `auto_sync_status` + `advance_status_along`.
- `src/routes/rolls.rs` — **modify** `roll_loaded` on create, `status_changed` on manual update, `events` in `RollDetail`/`get_detail`.
- `src/routes/shots.rs` — **modify** emit `shot_logged`/`shot_edited`/`shot_deleted`.
- `src/routes/development.rs` — **modify** emit `lab_dev_*` / `self_dev_*`.
- `tests/roll_events.rs` — **new** API-driven integration tests.

---

## Task 1: `roll_event` entity

**Files:**

- Create: `entity/src/roll_event.rs`
- Modify: `entity/src/lib.rs`

- [ ] **Step 1: Write the entity**

Create `entity/src/roll_event.rs`:

```rust
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::roll::RollStatus;

/// What kind of thing happened to a roll. Total enum — adding a variant forces
/// every match to be updated (mirrors RollStatus).
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum RollEventType {
    #[sea_orm(string_value = "roll_loaded")]
    #[serde(rename = "roll_loaded")]
    RollLoaded,
    #[sea_orm(string_value = "status_changed")]
    #[serde(rename = "status_changed")]
    StatusChanged,
    #[sea_orm(string_value = "shot_logged")]
    #[serde(rename = "shot_logged")]
    ShotLogged,
    #[sea_orm(string_value = "shot_edited")]
    #[serde(rename = "shot_edited")]
    ShotEdited,
    #[sea_orm(string_value = "shot_deleted")]
    #[serde(rename = "shot_deleted")]
    ShotDeleted,
    #[sea_orm(string_value = "lab_dev_added")]
    #[serde(rename = "lab_dev_added")]
    LabDevAdded,
    #[sea_orm(string_value = "lab_dev_edited")]
    #[serde(rename = "lab_dev_edited")]
    LabDevEdited,
    #[sea_orm(string_value = "lab_dev_removed")]
    #[serde(rename = "lab_dev_removed")]
    LabDevRemoved,
    #[sea_orm(string_value = "self_dev_added")]
    #[serde(rename = "self_dev_added")]
    SelfDevAdded,
    #[sea_orm(string_value = "self_dev_edited")]
    #[serde(rename = "self_dev_edited")]
    SelfDevEdited,
    #[sea_orm(string_value = "self_dev_removed")]
    #[serde(rename = "self_dev_removed")]
    SelfDevRemoved,
}

/// What record `ref_id` points to, so the frontend journal can deep-link an
/// event to its editor.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum RefKind {
    #[sea_orm(string_value = "lab_dev")]
    #[serde(rename = "lab_dev")]
    LabDev,
    #[sea_orm(string_value = "self_dev")]
    #[serde(rename = "self_dev")]
    SelfDev,
    #[sea_orm(string_value = "shot")]
    #[serde(rename = "shot")]
    Shot,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "roll_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub roll_id: i32,
    pub event_type: RollEventType,
    pub from_status: Option<RollStatus>,
    pub to_status: Option<RollStatus>,
    pub ref_kind: Option<RefKind>,
    pub ref_id: Option<i32>,
    pub summary: String,
    pub occurred_at: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::roll::Entity",
        from = "Column::RollId",
        to = "super::roll::Column::Id"
    )]
    Roll,
}

impl Related<super::roll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Roll.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

- [ ] **Step 2: Register the module**

In `entity/src/lib.rs`, add (keep alphabetical with siblings — after `pub mod roll;`):

```rust
pub mod roll_event;
```

- [ ] **Step 3: Build**

Run: `cargo build -p entity`
Expected: compiles clean.

- [ ] **Step 4: Commit**

```bash
git add entity/src/roll_event.rs entity/src/lib.rs
git commit -m "feat(entity): roll_event activity-log entity (kammerz-06i)"
```

---

## Task 2: create-table migration

**Files:**

- Create: `migration/src/m20260614_000022_create_roll_events.rs`
- Modify: `migration/src/lib.rs`

- [ ] **Step 1: Write the migration**

Create `migration/src/m20260614_000022_create_roll_events.rs`:

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RollEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RollEvents::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RollEvents::RollId).integer().not_null())
                    .col(ColumnDef::new(RollEvents::EventType).text().not_null())
                    .col(ColumnDef::new(RollEvents::FromStatus).text().null())
                    .col(ColumnDef::new(RollEvents::ToStatus).text().null())
                    .col(ColumnDef::new(RollEvents::RefKind).text().null())
                    .col(ColumnDef::new(RollEvents::RefId).integer().null())
                    .col(ColumnDef::new(RollEvents::Summary).text().not_null())
                    .col(ColumnDef::new(RollEvents::OccurredAt).text().not_null())
                    .col(ColumnDef::new(RollEvents::CreatedAt).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_roll_events_roll")
                            .from(RollEvents::Table, RollEvents::RollId)
                            .to(Rolls::Table, Rolls::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_roll_events_roll_id")
                    .table(RollEvents::Table)
                    .col(RollEvents::RollId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RollEvents::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum RollEvents {
    Table,
    Id,
    RollId,
    EventType,
    FromStatus,
    ToStatus,
    RefKind,
    RefId,
    Summary,
    OccurredAt,
    CreatedAt,
}

#[derive(Iden)]
enum Rolls {
    Table,
    Id,
}
```

- [ ] **Step 2: Register the migration**

In `migration/src/lib.rs`: add the `mod` line after `mod m20260602_000021_add_roll_lifecycle_dates;`:

```rust
mod m20260614_000022_create_roll_events;
```

and add to the `migrations()` vec, after the `m20260602_000021…` `Box::new`:

```rust
Box::new(m20260614_000022_create_roll_events::Migration),
```

- [ ] **Step 3: Verify migration applies (it runs in every test's DB init)**

Run: `cargo test -p kammerz --test health`
Expected: PASS (DB init runs all migrations incl. the new one; a failure here means the migration is malformed).

- [ ] **Step 4: Commit**

```bash
git add migration/src/m20260614_000022_create_roll_events.rs migration/src/lib.rs
git commit -m "feat(migration): create roll_events table (kammerz-06i)"
```

---

## Task 3: `RollEventService`

**Files:**

- Create: `src/services/roll_event_service.rs`
- Modify: `src/services/mod.rs`

- [ ] **Step 1: Write the service**

Create `src/services/roll_event_service.rs`:

```rust
use sea_orm::*;

use entity::roll::RollStatus;
use entity::roll_event::{self, RefKind, RollEventType};

use crate::patch::now_string;

pub struct RollEventService;

impl RollEventService {
    /// Append one event. Takes `&impl ConnectionTrait` so it runs inside the
    /// same transaction as the mutation it records (atomic with it).
    #[allow(clippy::too_many_arguments)]
    pub async fn record(
        db: &impl ConnectionTrait,
        roll_id: i32,
        event_type: RollEventType,
        from_status: Option<RollStatus>,
        to_status: Option<RollStatus>,
        ref_kind: Option<RefKind>,
        ref_id: Option<i32>,
        summary: String,
    ) -> Result<(), DbErr> {
        let now = now_string();
        let model = roll_event::ActiveModel {
            roll_id: Set(roll_id),
            event_type: Set(event_type),
            from_status: Set(from_status),
            to_status: Set(to_status),
            ref_kind: Set(ref_kind),
            ref_id: Set(ref_id),
            summary: Set(summary),
            occurred_at: Set(now.clone()),
            created_at: Set(now),
            ..Default::default()
        };
        model.insert(db).await?;
        Ok(())
    }

    /// Convenience for the most common event.
    pub async fn record_status_change(
        db: &impl ConnectionTrait,
        roll_id: i32,
        from: RollStatus,
        to: RollStatus,
    ) -> Result<(), DbErr> {
        let summary = format!("Status changed to {}", status_label(&to));
        Self::record(
            db,
            roll_id,
            RollEventType::StatusChanged,
            Some(from),
            Some(to),
            None,
            None,
            summary,
        )
        .await
    }

    /// Newest first. `id` desc tie-breaks events sharing an `occurred_at`.
    pub async fn list_for_roll(
        db: &impl ConnectionTrait,
        roll_id: i32,
    ) -> Result<Vec<roll_event::Model>, DbErr> {
        roll_event::Entity::find()
            .filter(roll_event::Column::RollId.eq(roll_id))
            .order_by_desc(roll_event::Column::OccurredAt)
            .order_by_desc(roll_event::Column::Id)
            .all(db)
            .await
    }
}

/// Human label for a status, for the denormalized `summary` fallback. The
/// frontend renders its own label from `to_status`; this keeps the row readable
/// without a frontend.
fn status_label(s: &RollStatus) -> &'static str {
    match s {
        RollStatus::Loaded => "Loaded",
        RollStatus::Shooting => "Shooting",
        RollStatus::Shot => "Shot",
        RollStatus::AtLab => "At Lab",
        RollStatus::LabDone => "Lab Done",
        RollStatus::Developing => "Developing",
        RollStatus::Developed => "Developed",
        RollStatus::Scanned => "Scanned",
        RollStatus::PostProcessed => "Post-processed",
        RollStatus::Archived => "Archived",
    }
}
```

- [ ] **Step 2: Register the module**

In `src/services/mod.rs`, add (alongside the other `pub mod` lines):

```rust
pub mod roll_event_service;
```

- [ ] **Step 3: Build**

Run: `cargo build -p kammerz`
Expected: compiles clean.

- [ ] **Step 4: Commit**

```bash
git add src/services/roll_event_service.rs src/services/mod.rs
git commit -m "feat(service): RollEventService for the activity log (kammerz-06i)"
```

---

## Task 4: expose `events` on the detail endpoint

**Files:**

- Modify: `src/routes/rolls.rs` (`RollDetail` struct + `get_detail`)
- Test: `tests/roll_events.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/roll_events.rs`:

```rust
mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json, put_json};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Create a roll on a seeded camera; return its id.
async fn create_roll(app: &axum::Router, roll_id: &str, status: &str) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;
    let payload = json!({
        "roll_id": roll_id,
        "camera_id": camera_id,
        "status": status,
        "date_loaded": "2026-05-01"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

async fn events_for(app: &axum::Router, roll_id: i32) -> Vec<Value> {
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_id}/detail")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let detail: Value = json_body(res).await;
    detail["events"].as_array().cloned().unwrap_or_default()
}

#[tokio::test]
async fn detail_includes_events_array() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-1", "loaded").await;
    // The events key must exist and be an array (creating a roll logs roll_loaded).
    let events = events_for(&app, id).await;
    assert!(
        events.iter().any(|e| e["event_type"] == "roll_loaded"),
        "expected a roll_loaded event, got: {events:?}"
    );
}
```

- [ ] **Step 2: Run it — verify it fails**

Run: `cargo test -p kammerz --test roll_events detail_includes_events_array`
Expected: FAIL — `events` key missing (and `roll_loaded` not emitted yet; emitted in Task 5).

- [ ] **Step 3: Add `events` to `RollDetail` and `get_detail`**

In `src/routes/rolls.rs`, add the field to the struct (after `dev_stages`):

```rust
pub dev_stages: Vec<dev_stage::Model>,
pub events: Vec<entity::roll_event::Model>,
```

In `get_detail`, before the `Ok(Json(RollDetail {` block, add:

```rust
let events = crate::services::roll_event_service::RollEventService::list_for_roll(&db, id).await?;
```

and add `events,` to the struct literal:

```rust
Ok(Json(RollDetail {
    roll,
    shots,
    shot_lens_pairs,
    lab_dev,
    self_dev,
    dev_stages,
    events,
}))
```

> The test still fails until Task 5 emits `roll_loaded`, but the `events` array now serializes. That's expected; we finish this test green in Task 5.

- [ ] **Step 4: Commit**

```bash
git add src/routes/rolls.rs tests/roll_events.rs
git commit -m "feat(api): add events to roll detail response (kammerz-06i)"
```

---

## Task 5: emit status + roll_loaded events

**Files:**

- Modify: `src/services/roll_service.rs` (`auto_sync_status`, `advance_status_along`)
- Modify: `src/routes/rolls.rs` (`create`, `update`)
- Test: `tests/roll_events.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests/roll_events.rs`:

```rust
#[tokio::test]
async fn manual_status_change_logs_event() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-2", "loaded").await;

    let res = app
        .clone()
        .oneshot(put_json(&format!("/api/rolls/{id}"), &json!({ "status": "shooting" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let events = events_for(&app, id).await;
    let sc = events
        .iter()
        .find(|e| e["event_type"] == "status_changed")
        .expect("expected a status_changed event");
    assert_eq!(sc["from_status"], "loaded");
    assert_eq!(sc["to_status"], "shooting");
}

#[tokio::test]
async fn events_are_newest_first() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-3", "loaded").await;
    app.clone()
        .oneshot(put_json(&format!("/api/rolls/{id}"), &json!({ "status": "shooting" })))
        .await
        .unwrap();
    let events = events_for(&app, id).await;
    // Most recent (status_changed) comes before the initial roll_loaded.
    assert_eq!(events.first().unwrap()["event_type"], "status_changed");
    assert_eq!(events.last().unwrap()["event_type"], "roll_loaded");
}
```

- [ ] **Step 2: Run — verify they fail**

Run: `cargo test -p kammerz --test roll_events`
Expected: FAIL — no status_changed/roll_loaded events yet.

- [ ] **Step 3: Emit `roll_loaded` on create**

In `src/routes/rolls.rs` `create`, replace the final return with an event emission first. After:

```rust
let result = RollService::create(&db, model)
    .await
    .map_err(|e| AppError::UnprocessableEntity(friendly_err("roll", e)))?;
```

add:

```rust
RollEventService::record(
    &db,
    result.id,
    entity::roll_event::RollEventType::RollLoaded,
    None,
    None,
    None,
    None,
    "Roll loaded".to_string(),
)
.await?;
```

(leave `Ok((StatusCode::CREATED, Json(result.id)))` as the return).

- [ ] **Step 4: Emit `status_changed` on manual update**

In `src/routes/rolls.rs` `update`, capture the previous + requested status. Right after `let existing = … .or_404("Roll", id)?;` add:

```rust
let prev_status = existing.status.clone();
let requested_status = data.status.clone();
```

Then after the `RollService::update(&db, model)…?;` call (before `Ok(StatusCode::NO_CONTENT)`), add:

```rust
if let Some(new_status) = requested_status {
    if new_status != prev_status {
        RollEventService::record_status_change(&db, id, prev_status, new_status).await?;
    }
}
```

- [ ] **Step 5: Emit `status_changed` inside the auto-sync funnels**

In `src/services/roll_service.rs` `auto_sync_status`, in the `if from_statuses.contains(&roll_record.status)` branch, capture the old status before consuming `roll_record` and emit after the update. Replace the branch body:

```rust
if from_statuses.contains(&roll_record.status) {
    let from = roll_record.status.clone();
    let now = now_string();
    let mut model: roll::ActiveModel = roll_record.into();
    model.status = Set(to_status.clone());
    model.updated_at = Set(now);
    model.update(db).await?;
    RollEventService::record_status_change(db, roll_id, from, to_status).await?;
    Ok(true)
} else {
    Ok(false)
}
```

In `advance_status_along`, the matching arm:

```rust
(Some(cur), Some(tgt)) if cur < tgt => {
    let from = roll_record.status.clone();
    let now = now_string();
    let mut model: roll::ActiveModel = roll_record.into();
    model.status = Set(target.clone());
    model.updated_at = Set(now);
    model.update(db).await?;
    RollEventService::record_status_change(db, roll_id, from, target).await?;
    Ok(true)
}
```

Add the import near the top of `roll_service.rs` (with the other `use crate::…` lines):

```rust
use crate::services::roll_event_service::RollEventService;
```

> `to_status` and `target` are `RollStatus` (now cloned because used after the move). `RollStatus` derives `Clone`.

- [ ] **Step 6: Add the `RollEventService` import to `rolls.rs`**

In `src/routes/rolls.rs`, with the other service imports:

```rust
use crate::services::roll_event_service::RollEventService;
```

- [ ] **Step 7: Run — verify green**

Run: `cargo test -p kammerz --test roll_events`
Expected: PASS (`detail_includes_events_array`, `manual_status_change_logs_event`, `events_are_newest_first`).

- [ ] **Step 8: Commit**

```bash
git add src/services/roll_service.rs src/routes/rolls.rs tests/roll_events.rs
git commit -m "feat(api): emit roll_loaded + status_changed events (kammerz-06i)"
```

---

## Task 6: emit shot events

**Files:**

- Modify: `src/routes/shots.rs` (`create`, `update`, `delete_one`)
- Test: `tests/roll_events.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/roll_events.rs`:

```rust
async fn first_shot(app: &axum::Router, roll_id: i32, frame: &str) -> i32 {
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/shots",
            &json!({ "roll_id": roll_id, "frame_number": frame, "date": "2026-05-02" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

#[tokio::test]
async fn logging_a_shot_logs_shot_and_autosync_events() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-4", "loaded").await;
    let shot_id = first_shot(&app, id, "1").await;

    let events = events_for(&app, id).await;
    let shot_ev = events
        .iter()
        .find(|e| e["event_type"] == "shot_logged")
        .expect("expected a shot_logged event");
    assert_eq!(shot_ev["ref_kind"], "shot");
    assert_eq!(shot_ev["ref_id"], shot_id);

    // First shot auto-advances loaded → shooting, which must also be logged.
    assert!(
        events.iter().any(|e| e["event_type"] == "status_changed"
            && e["from_status"] == "loaded"
            && e["to_status"] == "shooting"),
        "expected auto-sync status_changed loaded→shooting, got: {events:?}"
    );
}
```

- [ ] **Step 2: Run — verify it fails**

Run: `cargo test -p kammerz --test roll_events logging_a_shot_logs_shot_and_autosync_events`
Expected: FAIL — no `shot_logged` event (the `status_changed` half already passes from Task 5).

- [ ] **Step 3: Emit `shot_logged` in `create`**

In `src/routes/shots.rs` `create`, capture the frame label before it's moved into the model. Immediately after `let now = now_string();` add:

```rust
let frame_label = data.frame_number.trim().to_string();
let roll_id = data.roll_id;
```

Inside the transaction closure, after the `RollService::auto_sync_status(…).await?;` call and before `Ok(result.id)`, add:

```rust
RollEventService::record(
    txn,
    roll_id,
    entity::roll_event::RollEventType::ShotLogged,
    None,
    None,
    Some(entity::roll_event::RefKind::Shot),
    Some(result.id),
    format!("Frame {frame_label} logged"),
)
.await?;
```

- [ ] **Step 4: Emit `shot_edited` in `update`**

In `src/routes/shots.rs` `update`, after `let existing = … .or_404("Shot", id)?;` capture:

```rust
let roll_id = existing.roll_id;
```

Inside the transaction closure, after `ShotService::update(txn, model).await?;` and the optional `set_lenses_for_shot`, before `Ok(())`, add:

```rust
RollEventService::record(
    txn,
    roll_id,
    entity::roll_event::RollEventType::ShotEdited,
    None,
    None,
    Some(entity::roll_event::RefKind::Shot),
    Some(id),
    "Shot edited".to_string(),
)
.await?;
```

- [ ] **Step 5: Emit `shot_deleted` in `delete_one`**

In `src/routes/shots.rs` `delete_one`, inside the transaction closure, after the `if remaining == 0 { … }` auto-revert block and before `Ok(())`, add:

```rust
RollEventService::record(
    txn,
    roll_id,
    entity::roll_event::RollEventType::ShotDeleted,
    None,
    None,
    None,
    None,
    format!("Frame {} deleted", shot_record.frame_number),
)
.await?;
```

(`roll_id` and `shot_record` are already bound earlier in the closure.)

- [ ] **Step 6: Add the import**

In `src/routes/shots.rs`, with the other service imports:

```rust
use crate::services::roll_event_service::RollEventService;
```

- [ ] **Step 7: Run — verify green**

Run: `cargo test -p kammerz --test roll_events`
Expected: PASS (all prior + the new shot test).

- [ ] **Step 8: Commit**

```bash
git add src/routes/shots.rs tests/roll_events.rs
git commit -m "feat(api): emit shot_logged/edited/deleted events (kammerz-06i)"
```

---

## Task 7: emit development events

**Files:**

- Modify: `src/routes/development.rs` (`create_lab_dev`, `update_lab_dev`, `delete_lab_dev`, `create_self_dev`, `update_self_dev`, `delete_self_dev`)
- Test: `tests/roll_events.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/roll_events.rs`:

```rust
#[tokio::test]
async fn creating_lab_dev_logs_event() {
    let app = open_app().await;
    let id = create_roll(&app, "EVT-5", "shot").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": id, "date_dropped_off": "2026-05-10" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let lab_id: i32 = json_body(res).await;

    let events = events_for(&app, id).await;
    let ev = events
        .iter()
        .find(|e| e["event_type"] == "lab_dev_added")
        .expect("expected a lab_dev_added event");
    assert_eq!(ev["ref_kind"], "lab_dev");
    assert_eq!(ev["ref_id"], lab_id);
}
```

> Confirm the lab-create payload/route against `src/routes/development.rs` (`create_lab_dev`, registered at `POST /api/development/lab`) while implementing; adjust the JSON keys if the DTO differs.

- [ ] **Step 2: Run — verify it fails**

Run: `cargo test -p kammerz --test roll_events creating_lab_dev_logs_event`
Expected: FAIL — no `lab_dev_added` event.

- [ ] **Step 3: Emit the six dev events**

In `src/routes/development.rs`, add the import with the other service imports:

```rust
use crate::services::roll_event_service::RollEventService;
```

In each handler's success path (inside its transaction where one exists; otherwise right before the `Ok(...)` return), emit the matching event. `roll_id` is on the DTO for creates and on the loaded record for updates/deletes; the dev record's id is the create result / the path `id`. Use these exact calls:

`create_lab_dev` (after the dev is created + status synced; `roll_id` = DTO, `<new_id>` = created lab dev id):

```rust
RollEventService::record(
    txn, // or &db if the handler isn't transactional — match the surrounding code
    roll_id,
    entity::roll_event::RollEventType::LabDevAdded,
    None,
    None,
    Some(entity::roll_event::RefKind::LabDev),
    Some(new_id),
    "Lab development added".to_string(),
)
.await?;
```

`update_lab_dev` → `RollEventType::LabDevEdited`, `RefKind::LabDev`, `ref_id = id`, summary `"Lab development edited"`.
`delete_lab_dev` → `RollEventType::LabDevRemoved`, `ref_kind = None`, `ref_id = None`, summary `"Lab development removed"`.
`create_self_dev` → `RollEventType::SelfDevAdded`, `RefKind::SelfDev`, `ref_id = <new_id>`, summary `"Self development added"`.
`update_self_dev` → `RollEventType::SelfDevEdited`, `RefKind::SelfDev`, `ref_id = id`, summary `"Self development edited"`.
`delete_self_dev` → `RollEventType::SelfDevRemoved`, `ref_kind = None`, `ref_id = None`, summary `"Self development removed"`.

For update/delete handlers, capture `let roll_id = <loaded record>.roll_id;` from the existing record fetch (these handlers load the dev record before mutating — reuse that). Pass the same connection handle (`txn` inside a transaction, `&db` otherwise) the surrounding mutation uses.

- [ ] **Step 4: Run — verify green**

Run: `cargo test -p kammerz --test roll_events`
Expected: PASS (all tests).

- [ ] **Step 5: Commit**

```bash
git add src/routes/development.rs tests/roll_events.rs
git commit -m "feat(api): emit lab/self development events (kammerz-06i)"
```

---

## Task 8: full gate

- [ ] **Step 1: Run the whole backend suite + build**

Run: `cargo test -p kammerz`
Expected: PASS (no regressions in rolls/shots/development suites from the emission edits).

- [ ] **Step 2: Run the full CI mirror**

Run: `just ci`
Expected: `✅ just ci: all CI jobs passed`. Restore `frontend/build/.gitkeep` if the frontend build wiped it (`git checkout -- frontend/build/.gitkeep`).

- [ ] **Step 3: Open the PR**

```bash
git push -u origin <branch>
gh pr create --title "feat(api): roll activity log — Phase 1 (kammerz-06i)" --body "<summary + just ci result>"
```

Post the `just ci` output as the PR gate comment (GitHub Actions is unavailable).

---

## Self-review notes

- **Spec coverage:** `roll_events` table ✓ (T2), entity ✓ (T1), `RollEventService` ✓ (T3), event emission across roll/shot/dev ✓ (T5–T7), `events` on detail endpoint ✓ (T4), no backfill ✓ (migration is table-only), backend tests ✓ (T4–T8). Taxonomy: `roll_loaded`, `status_changed`, `shot_*`, `lab_dev_*`, `self_dev_*` all emitted; `roll_edited` is in the entity enum but intentionally **not** emitted in Phase 1 (conservative — add later if the journal needs it).
- **Type consistency:** `RollEventService::record(...)` signature is identical across all call sites; `RollEventType`/`RefKind` variant names match the entity. `record_status_change` used in both auto-sync funnels and the manual update handler.
- **Status emission completeness:** every status write funnels through `auto_sync_status`, `advance_status_along`, or the manual `update` handler — all three emit. Import (`import_roll`) inserts status directly and is intentionally event-free (bulk import, not interactive).
