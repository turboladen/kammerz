# Kammerz roll-data import tooling

One-time import of historical rolls from Apple Notes / Numbers / NotePlan.
See `docs/superpowers/plans/2026-06-15-roll-data-import.md` for the full procedure.

- `export-notes.applescript` — dumps a Notes folder to `~/kammerz-import/corpus/notes-export.txt`
  (create the output dir first: `mkdir -p ~/kammerz-import/corpus`)

Personal data (note corpus, CSV, staging, lookup) lives in `~/kammerz-import/` and is
NEVER committed (this repo is public).
