# ADR-0012: Canonical chemistry reference with self-learning autocomplete

- **Status:** Accepted
- **Date:** 2026-07-14
- **Related:** `entity/src/chemical.rs`, `src/services/chemical_service.rs`
  (`list_grouped`, `upsert_from_self_dev`), `src/routes/development.rs`
  (`GET /api/development/chemicals` + the auto-upsert in the self-dev create/update
  transactions), `migration/src/m20260713_000028_create_chemicals.rs`,
  `migration/src/m20260713_000029_normalize_dev_chemistry.rs`,
  `frontend/src/lib/components/rolls/DevelopmentSection.svelte`,
  `frontend/src/lib/components/ui/ComboInput.svelte` (`onselect`),
  `frontend/src/lib/utils/chemistry.ts`, `.claude/rules/backend-patterns.md`,
  beads `kammerz-9fx` (PR #134), follow-up `kammerz-ysk4`

## Context

Self-development records store `developer`, `fixer`, `stop_bath`, `wetting_agent`,
and `clearing_agent` as free text (`entity/src/development_self.rs`), entered as
plain inputs. The text drifted: `XTOL` vs `Kodak X-tol`, `D76` vs `Kodak D-76`,
`Photo-Flo` vs `Kodak Photo-Flo 200`. The 2026-06 import normalized 65 fields across
54 records to canonical names, but nothing prevented future drift and a fresh DB had
no canonical products seeded — so the same catalog value could be spelled three ways
across rolls, defeating grouping and recall.

Three mechanisms were considered:

- **Static suggestion list** (a build-time TS constant feeding autocomplete) —
  cheapest, but the list is fixed at build time, so a new product isn't remembered
  without a code change.
- **Derive suggestions from existing DB values** — self-extending with no new table,
  but a typo you enter once immediately becomes a first-class suggestion (drift is
  reinforced, not prevented).
- **Full FK normalization** of the chemistry columns to a lookup table — rigid, a
  heavier table-rebuild migration, and it loses the ability to type an ad-hoc value.

## Decision

A **seeded DB reference table + autocomplete**, with the `development_selves`
chemistry columns kept as **free-text strings (no FK)** so ad-hoc values still work.
The reference is **self-learning**.

- **Schema (migration 028):** `chemicals(id, name, type, default_dilution,
  created_at, updated_at, UNIQUE(name, type))`; `type` is a `ChemicalType`
  `DeriveActiveEnum` (developer/fixer/stop_bath/wetting_agent/clearing_agent). The
  Rust-keyword column is handled with `#[sea_orm(column_name = "type")]
  #[serde(rename = "type")] pub r#type` so the DB column and JSON/TS field stay
  `type`. Seeded (`INSERT OR IGNORE`) with the canonical products, their names
  byte-exact the normalization targets.
- **Self-learning, not a CRUD screen:** saving a self-dev `INSERT OR IGNORE`s each
  non-empty chemistry value into `chemicals` (with its column's type,
  `default_dilution` NULL) on the **same transaction** as the dev write
  (`upsert_from_self_dev`). Typing a new product once makes it a future suggestion —
  no admin UI.
- **Normalize existing rows (migration 029):** a separate idempotent migration maps
  the drifted values to the canonical seed names, driven by a `pub const
  NORMALIZATIONS` + `pub` apply-fn shared with its test so the two can't drift
  (see `.claude/rules/backend-patterns.md`).
- **Frontend:** the five chemistry inputs become `ComboInput`s fed from
  `GET /api/development/chemicals` (grouped by type); free text is preserved.
  Selecting a known developer/fixer pre-fills an **empty** dilution from
  `default_dilution` (never overwrites a non-empty one).

Deliberately **not** done: an FK on the chemistry columns (would kill free-text
flexibility), and a manage-chemicals CRUD/curation page (auto-upsert covers
extension; YAGNI).

## Consequences

- **Positive:** entries converge on canonical names without losing ad-hoc entry;
  a new product persists automatically as a future suggestion; a fresh DB ships with
  the canonical set; existing drifted rows are normalized on deploy.
- **Positive:** the chemistry columns stay simple free-text strings — no join, no FK
  migration, no rigidity; the reference table is purely advisory.
- **Negative (accepted):** self-learning has **no curation/delete path** — a typo
  saved once becomes a permanent autocomplete suggestion with no UI to remove it
  (`kammerz-ysk4`, deferred). The trade is deliberate: auto-upsert avoids a whole
  CRUD surface; suggestion hygiene can be added later if it matters.
- **Negative (minor):** `chemicals.default_dilution` is stored but currently read
  only for the dilution prefill; `created_at`/`updated_at` are carried for
  consistency but unused. Reference-table overhead, not a defect.
- Any new chemistry-bearing write path must call `upsert_from_self_dev` (or an
  equivalent) to keep the reference learning; forgetting it silently stops new
  values from becoming suggestions (no compile error).
