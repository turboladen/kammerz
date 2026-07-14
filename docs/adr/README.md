# Architecture Decision Records

This directory holds **ADRs** — short, immutable records of a significant decision:
the context that forced it, the choice made, and the consequences. They answer
"why is it this way?" for the next person (usually future-you). ADRs are the
**durable decision record** for this project.

## Why ADRs

A decision's current relevance is explicit in its **Status**: an `Accepted` decision
is live; a reversed one becomes `Superseded by ADR-N`, and the replacement points
back with `Supersedes ADR-M`. You never edit a decision's history — you supersede
it. Grep a topic and the status tells you immediately whether you're looking at
current truth or a retired call (see the `date_fuzzy` chain, [0003](0003-fuzzy-date-field.md)
→ [0004](0004-remove-fuzzy-dates.md)).

## Relationship to `docs/superpowers/` design docs

Design and planning still happen through the brainstorming / writing-plans workflow,
which produces a spec + implementation plan. Those are **transient working
artifacts** — scaffolding useful while a feature is in flight — **not** a permanent
archive. The durable _decision_ inside a design gets promoted to an ADR here; the
spec/plan itself is retired once the feature ships (it survives in git history if
ever needed). The earlier committed `docs/superpowers/specs`/`plans` archive was
retired this way — its durable decisions were distilled into ADRs 0002–0009.

## When to write one

Write an ADR when a decision is **cross-cutting / architectural** OR **likely to be
revisited or reversed** — a data-model convention, a framework choice, a policy that
spans many files. Not every feature needs one: a single-page UI refactor whose
conventions already live in code + `.claude/rules/` does not.

Rule of thumb: if reversing it later would need a "why did we do that?" explanation,
it's an ADR.

## Conventions

- One file per decision: `NNNN-kebab-title.md`, numbered sequentially.
- Format: a status block + **Context / Decision / Consequences** (Nygard style).
- **Immutable once `Accepted`.** To change a decision, add a new ADR that
  `Supersedes` it and flip the old one's status to `Superseded by` — only the status
  line of a superseded ADR is edited, never its body.
- `Status` values: `Accepted`, `Superseded by ADR-N`, `Deprecated`, `Proposed`.
- Reference durable artifacts (code paths, `.claude/rules/`, `UI_DESIGN.md`, beads),
  not transient design docs.

## Index

| ADR                                                       | Status                                               | Decision                                                              |
| --------------------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------- |
| [0001](0001-record-architecture-decisions.md)             | Accepted                                             | Adopt ADRs for architecture/decision records                          |
| [0002](0002-axum-sveltekit-spa-architecture.md)           | Accepted                                             | axum + embedded SvelteKit SPA, single self-hosted binary              |
| [0003](0003-fuzzy-date-field.md)                          | Superseded by [ADR-0004](0004-remove-fuzzy-dates.md) | Free-text `date_fuzzy` field for imprecise dates                      |
| [0004](0004-remove-fuzzy-dates.md)                        | Accepted                                             | Remove `date_fuzzy`; concrete best-guess date + notes annotation      |
| [0005](0005-24-hour-time.md)                              | Accepted                                             | Standardize all time display/entry on 24-hour format                  |
| [0006](0006-accessible-contrast-colorblind-safe-theme.md) | Accepted                                             | Accessible-contrast + colorblind-safe theme tokens                    |
| [0007](0007-data-driven-roll-status-sync.md)              | Accepted                                             | Data-driven roll status: auto-sync from milestone dates + dev records |
| [0008](0008-roll-detail-two-pane-activity-log.md)         | Accepted                                             | Roll detail: chevron status control + append-only activity log        |
| [0009](0009-negatives-pickup-derived-state.md)            | Accepted                                             | Negatives-pickup as derived state, parallel to the status machine     |
