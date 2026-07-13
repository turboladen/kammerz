# ADR-0001: Record architecture decisions with ADRs

- **Status:** Accepted
- **Date:** 2026-07-13
- **Related:** `docs/adr/README.md`, `.claude/rules/workflow.md`

## Context

Design and planning artifacts live under `docs/superpowers/specs/` and
`plans/` — thorough, dated, and produced per-feature by the brainstorming /
writing-plans workflow. They serve their moment well, but as durable records they
have a gap: a reader can't tell whether a dated design doc still reflects how the
app works. When a decision later reverses (e.g. a data-model field is removed), the
old spec keeps asserting the old design with no signal that it's stale, and
rewriting it to match the present would erase the historical record of _why_ the
original call was made.

We wanted a lightweight way to record durable decisions such that each one
self-declares its current relevance, and reversals are captured as an explicit
chain rather than by editing history.

## Decision

Adopt **Architecture Decision Records** in `docs/adr/`, numbered sequentially, in
lightweight Nygard style (a status block + Context / Decision / Consequences). An
ADR is **immutable once Accepted**; a decision changes by adding a new ADR that
`Supersedes` the old one, flipping the old one's `Status` to `Superseded by`.

ADRs complement the brainstorming / writing-plans workflow: feature design still
starts as a `docs/superpowers/` spec, but that spec is a **transient** working
artifact — its durable _decision_ is promoted to an ADR and the spec is retired once
the feature ships (it stays in git history). See
`docs/adr/README.md` for when to write one and the conventions.

## Consequences

- **Positive:** every decision's status is explicit; reversals read as a chain
  (`0003` → `0004`) with no revisionism; grepping a topic lands on a doc that says
  whether it's current. The barrier to recording a decision is low (~1 page).
- **Positive:** the previously-committed `docs/superpowers/specs`/`plans` archive is
  retired — its durable decisions distilled into ADRs 0002–0009 — rather than kept as
  a parallel, drift-prone doc set. Going forward those design docs are transient
  working artifacts (see this directory's README), not a permanent archive.
- **Negative / cost:** a second doc type to keep in mind, and the discipline to
  supersede rather than edit. Mitigated by the "only for cross-cutting or
  reversible decisions" rule — ADRs stay few and high-signal.
- This ADR is itself the record of the decision to adopt ADRs (bootstrap).
