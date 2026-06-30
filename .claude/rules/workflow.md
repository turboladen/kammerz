# Workflow & Reference

## Git / Branch Hygiene

- **PRs are squash-merged.** `git branch --merged origin/main` therefore _lies_ — squash breaks ancestry, so merged branches show as unmerged. Verify with `gh pr list --state merged --json headRefName` before deleting. GitHub doesn't auto-delete branches on merge, so remote branches accumulate; prune with `git push origin --delete <branch>`.
- **Branch protection requires only the `all-checks` job** (the `ci.yml` aggregator), not the individual `format`/`backend`/`frontend`/`e2e` jobs. This is a **manual GitHub-settings step** (it cannot be set from the workflow): Settings → Branches → branch protection rule for `main` → "Require status checks to pass" → add **`all-checks`** and **remove** any individually-required `format`/`backend`/`frontend`/`e2e` entries. Required because `e2e` is now path-filtered (skipped on docs/beads-only PRs) — a directly-required check that gets skipped wedges the merge, whereas the skip-aware `all-checks` passes through a legitimately-skipped `e2e` while still failing on any real `failure`/`cancelled`.
- **Re-gate `main` after every merge in a PR train.** Squash merges can collide semantically with zero textual conflict (PRs #35 + #48 each added a different `open_app_with_db` to `tests/common/mod.rs`; `main` stopped compiling — hotfixed in #54). After each merge, run `just ci` (or at minimum `cargo test`) on updated `main` before merging the next PR; GitHub's `mergeable` only checks text.
- When several PRs in a train append tests to the **same** test file (e.g. `tests/rolls.rs`), expect a both-added textual conflict at gate time even though each test is independent — resolve by keeping both, no rebase drama.
- Mechanical/formatting PRs conflict with everything: merge them **last** in a train, regenerating the `just fmt` commit against final `main` right before merge.
- `gh pr view --json mergeable` returns `UNKNOWN` while GitHub recomputes after a push/merge — poll until it settles before trusting it.

## Reference

- Another SeaORM + SQLite project by the same author: `~/Development/projects/financier` (same SeaORM patterns). The axum + tower-sessions + rust-embed server structure mirrors `~/Development/projects/chorez`.
- `UI_DESIGN.md` documents the visual design system (colors, typography, components, layout).
- Project rules live in `.claude/rules/*.md`, imported from the root `CLAUDE.md`. The Beads issue-tracker block in `CLAUDE.md` is managed by `bd` (between its `BEGIN/END BEADS INTEGRATION` markers) — don't hand-edit it or move it into a rule file.
