# Architecture Decision Records

This directory holds **ADRs** — short, immutable records of a significant decision:
the context that forced it, the choice made, and the consequences. They answer
"why is it this way?" for the next person (usually future-you).

## Why ADRs (vs. the design docs in `docs/superpowers/`)

`docs/superpowers/specs/` and `plans/` capture the _full_ design and step-by-step
implementation of a feature at a point in time. They're dated and, once the work
ships, historical — a spec can silently drift from the code as decisions evolve.

An ADR captures the _durable decision_, and its **Status** makes its current
relevance explicit: an `Accepted` decision is live; a reversed one becomes
`Superseded by ADR-N` and the replacement points back with `Supersedes ADR-M`. You
never edit a decision's history — you supersede it. Grep a topic and the status
tells you immediately whether you're looking at current truth or a retired call.

## When to write one

Write an ADR when a decision is **cross-cutting / architectural** OR **likely to be
revisited or reversed** — e.g. a data-model convention, a framework choice, a
policy that spans many files. Do _not_ write one for every feature: feature-level
design still lives in a `docs/superpowers/` spec. When a spec contains a durable
decision, **promote that decision to an ADR** and leave the spec as the fuller
record.

Rule of thumb: if reversing it later would need a "why did we do that?" explanation,
it's an ADR.

## Conventions

- One file per decision: `NNNN-kebab-title.md`, numbered sequentially (not
  date-prefixed like the specs — ADRs reference each other by number).
- Format: a status block + **Context / Decision / Consequences** (Nygard style).
- **Immutable once `Accepted`.** To change a decision, add a new ADR that
  `Supersedes` it and flip the old one's status to `Superseded by`. Only the
  status line of a superseded ADR is edited — never its body.
- `Status` values: `Accepted`, `Superseded by ADR-N`, `Deprecated`, `Proposed`.

## Index

| ADR                                             | Status                                           | Decision                                                         |
| ----------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------- |
| [0001](0001-record-architecture-decisions.md)   | Accepted                                         | Adopt ADRs for architecture/decision records                     |
| [0002](0002-axum-sveltekit-spa-architecture.md) | Accepted                                         | axum + embedded SvelteKit SPA, single self-hosted binary         |
| [0003](0003-fuzzy-date-field.md)                | Superseded by [0004](0004-remove-fuzzy-dates.md) | Free-text `date_fuzzy` field for imprecise dates                 |
| [0004](0004-remove-fuzzy-dates.md)              | Accepted                                         | Remove `date_fuzzy`; concrete best-guess date + notes annotation |
| [0005](0005-24-hour-time.md)                    | Accepted                                         | Standardize all time display/entry on 24-hour format             |
