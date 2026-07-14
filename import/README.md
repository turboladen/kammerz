# Kammerz roll-data import tooling

Tooling for the one-time, agent-driven import of historical rolls from Apple Notes
into the catalog (now complete — 300+ rolls landed). The step-by-step runbook was a
transient working doc; this exporter is the reusable piece worth keeping.

- `export-notes.applescript` — dumps a Notes folder to `~/kammerz-import/corpus/notes-export.txt`
  (create the output dir first: `mkdir -p ~/kammerz-import/corpus`)

Personal data (note corpus, CSV, staging, lookup) lives in `~/kammerz-import/` and is
NEVER committed (this repo is public).
