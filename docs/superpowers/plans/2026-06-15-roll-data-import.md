# Roll Data Import Implementation Plan

> **Historical note (preserved into `main` after the fact, 2026-07):** This plan predates
> the removal of `date_fuzzy` (epic `kammerz-qdk`); its fuzzy-date handling is superseded by
> the concrete-date + notes-annotation convention (reconciliation tracked by `kammerz-31c` /
> `kammerz-btv`). The one-time 300+ roll import it describes is complete. Kept for the record.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Get ~300+ rolls of historical film data (Apple Notes 300+, Numbers ~36, NotePlan likely-skipped) into Kammerz accurately, via the app's existing REST endpoints, with human review at the parse step.

**Architecture:** One-time, agent-driven batch import. Bulk-export each source to a local corpus → Claude parses + matches against the catalog → writes via existing endpoints → human reviews staging docs before each write. Two phases: (1) reconcile the gear _vocabulary_ (cameras/lenses/film-stocks/labs) and create it once, recording a `name → id` lookup; (2) parse rolls in batches that resolve gear via that lookup. Idempotent/resumable via the UNIQUE `roll_id` index.

**Tech Stack:** macOS AppleScript (`osascript`) for Notes export; Numbers CSV export; `curl` against axum dev server on `:3002` (open auth in dev); Kammerz REST API; `bd` (beads) for cross-session progress; SQLite (`sqlite3`) for verification.

**Source spec:** `docs/superpowers/specs/2026-06-15-roll-data-import-design.md` (this branch).

---

## Ground rules (read before any task)

- **Public repo — never commit personal data.** Tooling (`import/*.applescript`, `import/*.sh`, `import/README.md`) is tracked. The note corpus, Numbers CSV, staging docs, and the gear lookup live **outside the repo** in `~/kammerz-import/` and are never `git add`-ed.
- **Isolation:** all work happens in the `import-roll-data` worktree. **Keep the worktree alive** across sessions until the import is fully done — the gitignored corpus/lookup live only on local disk.
- **Dev target:** import into local `just dev` on `:3002`. Auth is open in dev (no `KAMMERZ_PASSWORD_HASH`), so no login/cookie is needed for `curl`.
- **Reuse endpoints; build no app code.** Skip `/api/import/parse` — Claude is the parser.
- **All imported rolls land `status: "archived"`** (historical).
- **Resumable:** `roll_id` is UNIQUE — re-POSTing an imported roll 422s; the beads tracker records what's done.

## API reference (existing endpoints, all under `http://localhost:3002`)

| Purpose                     | Method + path                 | Body shape (key fields)                                                                                                                                                                                                                           |
| --------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Health                      | `GET /api/health`             | — (200 + `{sha,…}`)                                                                                                                                                                                                                               |
| List/Create lens mount      | `GET/POST /api/lens-mounts`   | `{ name, format_family? }`                                                                                                                                                                                                                        |
| List/Create film stock      | `GET/POST /api/film-stocks`   | `{ brand, name, iso?, format?, … }`                                                                                                                                                                                                               |
| List/Create lab             | `GET/POST /api/labs`          | `{ name, … }`                                                                                                                                                                                                                                     |
| List/Create lens            | `GET/POST /api/lenses`        | `{ brand, model, lens_mount_id, focal_length?, max_aperture?, … }`                                                                                                                                                                                |
| List/Create camera (normal) | `GET/POST /api/cameras`       | `{ brand, model, prefix?, lens_mount_id, default_lens_id?, … }`                                                                                                                                                                                   |
| Create fixed-lens camera    | `POST /api/cameras/with-lens` | `{ camera:{…,lens_mount_id}, lens:{…} }`                                                                                                                                                                                                          |
| Import roll + shots         | `POST /api/import/roll`       | see below                                                                                                                                                                                                                                         |
| Create self-dev record      | `POST /api/development/self`  | `{ roll_id(i32), date_processed?, developer?, developer_dilution?, fixer?, fixer_dilution?, stop_bath?, wetting_agent?, clearing_agent?, temperature?, agitation_notes?, notes?, stages?:[{stage_name, duration_seconds?, notes?, sort_order}] }` |

`POST /api/import/roll` body (from `src/routes/import.rs::ImportRollDto`):

```json
{
	"roll_id": "LR6-12",
	"camera_id": 4,
	"film_stock_id": 7,
	"lens_id": 9,
	"status": "archived",
	"frame_count": 36,
	"date_loaded": "2019-06-01",
	"date_finished": "2019-07-15",
	"date_fuzzy": "Summer 2019",
	"push_pull": null,
	"notes": "roll-level notes",
	"shots": [
		{
			"frame_number": "1",
			"aperture": "f/5.6",
			"shutter_speed": "1/125",
			"date": "2019-06-01",
			"date_fuzzy": null,
			"location": "Coast",
			"notes": "shot note",
			"lens_ids": [9]
		}
	]
}
```

Validation the endpoint enforces (so the parser must respect it): `roll_id` non-empty; `frame_count` non-negative; every shot `frame_number` non-empty and **unique within the roll**; dates that are present must be `YYYY-MM-DD`. Absent/unknown values → omit or `null` (use `date_fuzzy` for vague dates).

**Exact field names are confirmed before execution** in Task 0.6 by reading the live DTOs — endpoints evolve.

---

## Calibration addendum (confirmed from the 309-note export, 2026-06-18)

Full findings: `~/kammerz-import/staging/calibration-notes.md`. Parser-affecting facts:

- **Source confirmed:** 309 notes from `Hobbies / Photo Log`. Detect rolls by TITLE `^[A-Za-z0-9]+ ?[A-Za-z0-9]*-[0-9]+` **plus** a film-stock line 2; everything else (≈15–20 notes: "Repair Shops", "Rokkor lenses", "Developer Mix", "Sunny 16", …) is **filtered out** — but harvest the catalog notes (Rokkor lenses, Minolta XD-11) into Phase 1 vocabulary.
- **Note layout (positional):** L1 roll-ID, L2 film stock, `Loaded M/D/YY`, optional roll/lens/push notes, `Finished M/D/YY`, optional self-dev block, then shots.
- **Shots are positional — NO explicit frame numbers.** Assign `frame_number` sequentially by body order (1..N). `same` inherits prior focal/aperture/shutter; `?` = unknown frame (still a position). 35mm often sparse; medium format usually full.
- **Focal length → `lens_id`.** Shot lines carry `65mm`/`90mm`; the payload wants `lens_ids`. Phase 1 MUST catalog each camera's lenses with focal lengths so the parser can map focal → lens_id. Mid-roll swaps set `lens_ids` from that frame onward.
- **Dates are `M/D/YY` / `M/D`** (2-digit or implied year). Convert to `YYYY-MM-DD`; vague → `date_fuzzy`. `CREATED`/`MODIFIED` are fallbacks.
- **Trailing noise:** `Process`, `Moved to` (NotePlan migration breadcrumb — confirms NotePlan is the abandoned copy).
- **~30 camera prefixes** to map in Phase 1 (M67 24, OX 23, MXD7 20, MZM 19, CAX 16, LR6 14, … + singletons).

### Scope change: self-development records (user chose highest fidelity)

Every **developed** roll gets a SECOND write after the roll is created: `POST /api/development/self` with the roll's **numeric id** + the parsed dev block (developer, dilution, fixer, stop_bath, wetting_agent=Photo-flo, temperature, `stages[]` with `duration_seconds`). Almost all self-dev rolls are B&W. Because rolls are imported `archived` (terminal, past `developed`), the dev-create's forward-only auto-advance is a no-op — no status conflict. This adds a per-roll dev sub-step to Tasks 2a.3 and 2b.2 (Step 4) below.

---

## Phase 0 — Tooling, export & recon

### Task 0.1: Scaffold the import workspace

**Files:**

- Create (tracked): `import/README.md`, `import/.gitkeep`
- Create (outside repo): `~/kammerz-import/{corpus,staging,lookup}/`

- [ ] **Step 1: Create the tracked tooling dir + runbook stub**

```bash
mkdir -p import
cat > import/README.md <<'EOF'
# Kammerz roll-data import tooling

One-time import of historical rolls from Apple Notes / Numbers / NotePlan.
See `docs/superpowers/plans/2026-06-15-roll-data-import.md` for the full procedure.

- `export-notes.applescript` — dumps a Notes folder to ~/kammerz-import/corpus/notes-export.txt
- `post-roll.sh` — POSTs a roll JSON file to the dev API

Personal data (note corpus, CSV, staging, lookup) lives in ~/kammerz-import/ and is
NEVER committed (this repo is public).
EOF
```

- [ ] **Step 2: Create the out-of-repo data dirs**

```bash
mkdir -p ~/kammerz-import/corpus ~/kammerz-import/staging ~/kammerz-import/lookup
```

- [ ] **Step 3: Commit the tooling scaffold**

```bash
git add import/README.md
git commit -m "chore(import): scaffold import tooling dir + runbook"
```

Expected: commit succeeds; `~/kammerz-import/` is untracked by git (it's outside the repo root).

---

### Task 0.2: Write the Apple Notes exporter

**Files:**

- Create (tracked): `import/export-notes.applescript`

- [ ] **Step 1: Write the AppleScript**

```applescript
-- export-notes.applescript
-- Dumps notes from a target folder to a delimited UTF-8 corpus file.
-- Edit `targetFolder` (set to "" to export every note), then run:
--   osascript import/export-notes.applescript
-- Output: ~/kammerz-import/corpus/notes-export.txt

property targetFolder : "Film Rolls" -- <-- set to your Notes folder name; "" = all notes

on run
	set outPath to (POSIX path of (path to home folder)) & "kammerz-import/corpus/notes-export.txt"
	set outFile to POSIX file outPath
	set fileRef to open for access outFile with write permission
	set eof of fileRef to 0
	set noteCount to 0
	tell application "Notes"
		if targetFolder is "" then
			set theNotes to every note
		else
			set theNotes to every note of folder targetFolder
		end if
		repeat with n in theNotes
			set noteCount to noteCount + 1
			set rec to "@@@NOTE@@@" & linefeed
			set rec to rec & "TITLE: " & (name of n) & linefeed
			try
				set rec to rec & "FOLDER: " & (name of container of n) & linefeed
			on error
				set rec to rec & "FOLDER: (none)" & linefeed
			end try
			set rec to rec & "CREATED: " & ((creation date of n) as string) & linefeed
			set rec to rec & "MODIFIED: " & ((modification date of n) as string) & linefeed
			set rec to rec & "@@@BODY@@@" & linefeed
			set rec to rec & (body of n) & linefeed
			set rec to rec & "@@@ENDNOTE@@@" & linefeed
			my appendText(fileRef, rec)
		end repeat
	end tell
	close access fileRef
	return "Exported " & noteCount & " notes to " & outPath
end run

on appendText(fileRef, t)
	write t to fileRef as «class utf8»
end appendText
```

- [ ] **Step 2: Commit the exporter**

```bash
git add import/export-notes.applescript
git commit -m "chore(import): Apple Notes -> corpus AppleScript exporter"
```

---

### Task 0.3: Calibration export (small folder) + format check

This validates the script and the parser's assumptions on a few notes **before** the full 300.

- [ ] **Step 1 (USER):** In Apple Notes, identify the folder holding the rolls. If it's huge, temporarily make a small test folder with ~3 representative roll notes (mix of detailed and sparse), and set `property targetFolder` in the script to that folder. Run via the `!` prefix:

```
! osascript import/export-notes.applescript
```

Expected output: `Exported 3 notes to /Users/<you>/kammerz-import/corpus/notes-export.txt`. macOS will prompt once to grant Terminal/osascript access to Notes — approve it.

- [ ] **Step 2: Inspect the corpus format**

```bash
head -60 ~/kammerz-import/corpus/notes-export.txt
```

Expected: `@@@NOTE@@@` / `TITLE:` / `FOLDER:` / `CREATED:` / `MODIFIED:` / `@@@BODY@@@` / HTML body / `@@@ENDNOTE@@@` per note.

- [ ] **Step 3: Calibrate the parser**

Read the 3 records. Confirm: (a) one note == one roll (or note the grouping if not); (b) the roll-ID is recoverable from the title; (c) shots are parseable from the HTML body; (d) `CREATED`/`MODIFIED` dates are usable date fallbacks. Write findings as a short note in `~/kammerz-import/staging/calibration-notes.md` (out of repo). If one-note ≠ one-roll, STOP and revise the plan's parsing assumption with the user.

---

### Task 0.4: Full Apple Notes export

- [ ] **Step 1 (USER):** Set `property targetFolder` back to the real rolls folder; run:

```
! osascript import/export-notes.applescript
```

Expected: `Exported <N> notes …` where N ≈ 300+.

- [ ] **Step 2: Sanity-count the corpus**

```bash
grep -c '@@@NOTE@@@' ~/kammerz-import/corpus/notes-export.txt
```

Expected: the same N. Record N — it's the denominator for import progress.

---

### Task 0.5: Numbers export

- [ ] **Step 1 (USER):** In Numbers, open the rolls doc → File → Export To → CSV → save to `~/kammerz-import/corpus/numbers-rolls.csv`. If multiple tables/sheets, export the rolls table.

- [ ] **Step 2: Inspect columns**

```bash
head -5 ~/kammerz-import/corpus/numbers-rolls.csv
```

Record the header row → defines the column→field mapping used in Task 2a.1.

---

### Task 0.6: Environment recon + DB backup

**Files:** read-only inspection of `src/routes/import.rs`, `src/routes/cameras.rs`, `src/routes/film_stocks.rs`, `src/routes/lenses.rs`, `src/routes/labs.rs`, `src/routes/lens_mounts.rs`.

- [ ] **Step 1 (USER):** Start the dev server in a separate terminal:

```
! just dev
```

- [ ] **Step 2: Confirm reachability + open auth**

```bash
curl -fsS http://localhost:3002/api/health && echo && curl -fsS http://localhost:3002/api/auth/me
```

Expected: health 200 with a JSON body; `/api/auth/me` shows `auth_required: false` (open mode). If `auth_required: true`, STOP — the dev env has a password hash set; resolve with the user before continuing.

- [ ] **Step 3: Back up the dev DB**

```bash
# DATABASE_URL dev default is sqlite:./kammerz.db?mode=rwc (in the MAIN checkout, not this worktree)
cp ../../kammerz.db ~/kammerz-import/kammerz.db.pre-import.bak 2>/dev/null \
  && echo "backed up" || echo "NOTE: confirm the dev DB path with the user"
```

Expected: a backup exists (or the user confirms the actual dev DB path; dev runs from the main checkout). This makes the whole import revertible.

- [ ] **Step 4: Confirm the live DTO field names**

Read `src/routes/import.rs` (`ImportRollDto`/`ImportShotDto`), `src/routes/cameras.rs` (`CreateCameraDto`, `CreateCameraWithLensDto`), and the create DTOs in `film_stocks.rs`, `lenses.rs`, `labs.rs`, `lens_mounts.rs`. Update the API-reference table at the top of this plan if any field name differs. (Endpoints evolve; the parser must match reality.)

- [ ] **Step 5: Snapshot the seed catalog baseline**

```bash
for p in lens-mounts film-stocks labs lenses cameras; do
  echo "== $p =="; curl -fsS http://localhost:3002/api/$p | python3 -m json.tool | head -40
done
```

Record what seed records already exist so Phase 1 matches against them instead of creating duplicates (the migrations seed common film stocks and lens mounts).

---

## Phase 1 — Vocabulary reconciliation

Goal: every camera/lens/film-stock/lab/mount the rolls reference exists in the DB, with a `name → id` lookup recorded at `~/kammerz-import/lookup/vocab.json`.

### Task 1.1: Extract distinct roll-ID prefixes → camera spine

- [ ] **Step 1: List candidate roll IDs + prefixes from the corpus**

Parse `~/kammerz-import/corpus/notes-export.txt`: from each note TITLE (and body, if the ID lives there), extract the roll-ID (e.g. `LR6-12`) and its prefix (`LR6`). Produce the distinct prefix set with an example ID + a count per prefix. Write to `~/kammerz-import/lookup/prefixes.md`.

- [ ] **Step 2 (USER review):** Confirm each prefix → camera mapping (e.g. `LR6` → Leica R6, `LSL` → Leica SL). Annotate any ambiguous/legacy prefixes. This is the spine of camera creation in Task 1.5 (each camera's `prefix` field is set so the existing prefix-match works).

### Task 1.2: Parse the half-finished catalog note → gear specs

- [ ] **Step 1:** Locate the camera/lens catalog note(s) in the corpus (likely titled "Cameras", "Gear", "Lenses", etc.). Parse into proposed records: cameras (brand, model, prefix, mount, serial, fixed-lens?), lenses (brand, model, mount, focal length, max aperture, serial). Write proposals to `~/kammerz-import/lookup/catalog-proposed.md`.

### Task 1.3: Sweep corpus for film stocks, labs, lens mentions

- [ ] **Step 1:** Scan all roll notes for distinct film stock names, lab names, and any lens not already in the catalog note. Dedup aggressively (normalize case/spelling; `M67` ≡ `Mamiya RB67`). Cross-reference against the seed catalog from Task 0.6 Step 5 — mark each as MATCH (existing id) or NEW (propose). Append to the proposal docs.

### Task 1.4: Vocabulary review doc

- [ ] **Step 1:** Assemble `~/kammerz-import/staging/vocab-review.md` with one markdown table per type (mounts, film stocks, labs, lenses [with mount], cameras [with prefix + fixed-lens flag]), each row marked MATCH `<id>` or NEW `<fields>`.

- [ ] **Step 2 (USER review):** Approve / edit / merge. Resolve every NEW row's fields and every duplicate. Do not proceed until the user signs off — this vocabulary is referenced by all 300+ rolls.

### Task 1.5: Create vocabulary records + build the lookup

Create in **FK order**: mounts → film stocks / labs → lenses → cameras. Record each returned id.

- [ ] **Step 1: Create lens mounts (only NEW rows)**

```bash
curl -fsS -X POST http://localhost:3002/api/lens-mounts \
  -H 'Content-Type: application/json' \
  -d '{"name":"Leica R"}'
# -> returns the new id (raw integer, no {data} wrapper)
```

- [ ] **Step 2: Create film stocks + labs (only NEW rows)**

```bash
curl -fsS -X POST http://localhost:3002/api/film-stocks \
  -H 'Content-Type: application/json' \
  -d '{"brand":"Kodak","name":"Portra 400","iso":400,"format":"135"}'
curl -fsS -X POST http://localhost:3002/api/labs \
  -H 'Content-Type: application/json' -d '{"name":"The Darkroom"}'
```

- [ ] **Step 3: Create lenses (only NEW rows; needs lens_mount_id from Step 1)**

```bash
curl -fsS -X POST http://localhost:3002/api/lenses \
  -H 'Content-Type: application/json' \
  -d '{"brand":"Leica","model":"Summicron-R 50mm","lens_mount_id":3,"focal_length":"50mm","max_aperture":"f/2"}'
```

- [ ] **Step 4: Create cameras — normal vs fixed-lens**

Normal camera (set `prefix` so roll prefix-matching works downstream):

```bash
curl -fsS -X POST http://localhost:3002/api/cameras \
  -H 'Content-Type: application/json' \
  -d '{"brand":"Leica","model":"R6","prefix":"LR6","lens_mount_id":3}'
```

Fixed-lens camera (point-and-shoot etc.) — transactional camera+lens+junction+default:

```bash
curl -fsS -X POST http://localhost:3002/api/cameras/with-lens \
  -H 'Content-Type: application/json' \
  -d '{"camera":{"brand":"Yashica","model":"T4","prefix":"YT4","lens_mount_id":1},"lens":{"brand":"Carl Zeiss","model":"Tessar 35mm","focal_length":"35mm","max_aperture":"f/3.5"}}'
```

- [ ] **Step 5: Build the lookup file**

Write `~/kammerz-import/lookup/vocab.json` mapping normalized names → ids for every type, e.g.:

```json
{
	"mounts": { "leica r": 3 },
	"film_stocks": { "kodak portra 400": 7 },
	"labs": { "the darkroom": 2 },
	"lenses": { "leica summicron-r 50mm": 9 },
	"cameras": { "LR6": 4, "YT4": 5 }
}
```

(Cameras keyed by **prefix** — the roll's natural join key.)

- [ ] **Step 6: Verify the catalog round-trips**

```bash
for p in lens-mounts film-stocks labs lenses cameras; do
  echo "== $p count =="; curl -fsS http://localhost:3002/api/$p | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))'
done
```

Expected: counts = seed + newly created. Spot-check a couple of records in the app UI (`http://localhost:5273`).

- [ ] **Step 7: Commit progress marker** (tooling/docs only — NOT the lookup)

```bash
git add docs/superpowers/plans/2026-06-15-roll-data-import.md
git commit -m "chore(import): phase 1 vocabulary reconciled (lookup built locally)"
```

---

## Phase 2a — Numbers dry-run (the easy 36, validates the pipeline)

### Task 2a.1: Define the CSV → roll mapping

- [ ] **Step 1:** From the header recorded in Task 0.5, write an explicit column→field map (e.g. `Roll ID → roll_id`, `Camera → prefix lookup`, `Film → film_stock lookup`, `Loaded → date_loaded`, `Notes → notes`). Record it in `~/kammerz-import/staging/numbers-mapping.md`. Flag columns with no Kammerz home (carry into roll `notes`).

### Task 2a.2: Parse CSV → roll JSONs + staging doc

- [ ] **Step 1:** For each CSV row, build a roll JSON (see API reference) into `~/kammerz-import/staging/numbers/<roll_id>.json`. Resolve `camera_id` via prefix lookup, `film_stock_id`/`lens_id` via name lookup. `status: "archived"`. Dates → `YYYY-MM-DD` or `date_fuzzy`. Numbers rows are usually shot-summary level, so `shots` may be empty or a small list — only include shots the sheet actually records.

- [ ] **Step 2:** Emit `~/kammerz-import/staging/numbers-review.md`: one section per roll showing source row + resolved JSON + flags (unmatched gear, bad dates, duplicate frame numbers).

- [ ] **Step 3 (USER review):** Approve / correct. Fix any unmatched gear (add to vocab + lookup if genuinely missing).

### Task 2a.3: Import + verify the Numbers rolls

- [ ] **Step 1: Write the POST helper**

```bash
cat > import/post-roll.sh <<'EOF'
#!/usr/bin/env bash
# Usage: import/post-roll.sh <roll.json>
# POSTs a roll to the dev import endpoint; prints HTTP status + body.
set -euo pipefail
f="$1"
code=$(curl -sS -o /tmp/post-roll.out -w '%{http_code}' \
  -X POST http://localhost:3002/api/import/roll \
  -H 'Content-Type: application/json' --data-binary @"$f")
echo "[$code] $(basename "$f"): $(cat /tmp/post-roll.out)"
EOF
chmod +x import/post-roll.sh
git add import/post-roll.sh && git commit -m "chore(import): add post-roll.sh helper"
```

- [ ] **Step 2: POST each approved roll**

```bash
for f in ~/kammerz-import/staging/numbers/*.json; do import/post-roll.sh "$f"; done
```

Expected: each prints `[201] <roll_id>: <new-id>`. A `[422]` means a validation/duplicate problem — read the message, fix the JSON, re-POST (idempotent: a true duplicate `roll_id` 422s, which is the resume backstop).

- [ ] **Step 3: Verify**

```bash
curl -fsS http://localhost:3002/api/rolls | python3 -c 'import sys,json;d=json.load(sys.stdin);print("rolls:",len(d))'
```

Expected: 36 (or however many CSV rows). Open 2–3 in the app and diff against the CSV. This confirms the end-to-end pipeline before the big batch.

---

## Phase 2b — Apple Notes batch import (the 300+)

The repeatable procedure. Tracked in beads so it resumes cleanly across sessions.

### Task 2b.1: Create the beads tracker

- [ ] **Step 1 (single session only — bd dislikes concurrent writes):**

```bash
bd create --title="Import historical rolls from Apple Notes (300+)" \
  --description="Batch import of the Apple Notes corpus via the agent-driven pipeline. See docs/superpowers/plans/2026-06-15-roll-data-import.md" \
  --type=task --priority=2
```

Record the returned id as the tracking epic. Use a checklist in `~/kammerz-import/staging/notes-progress.md` (roll_id → done/flagged) as the fine-grained tracker; bd holds the epic-level status.

### Task 2b.2: The batch loop (repeat until corpus exhausted)

For each batch, pick the next un-imported rolls from the corpus. **Confidence-tiered sizing:** ~20–25 rolls when they're clean and prefix-resolved; ~8–10 when sparse/messy/ambiguous.

- [ ] **Step 1 — Parse:** For each roll in the batch, parse its note (HTML body) → roll JSON in `~/kammerz-import/staging/notes/<roll_id>.json`. Resolve gear via `vocab.json` (camera by prefix; film/lens by name). Use `CREATED`/`MODIFIED` as date fallbacks (`date_fuzzy` when only a vague date is known). Shots are **positional** — assign `frame_number` 1..N by body order (`same` inherits prior values; `?` is a sparse frame); resolve each shot's focal length to `lens_ids`. If the note has a self-dev block, also write `~/kammerz-import/staging/notes/<roll_id>.dev.json` (CreateSelfDevDto minus `roll_id`, which is injected post-create).

- [ ] **Step 2 — Stage:** Emit `~/kammerz-import/staging/notes-batch-NN.md`: per roll, source body + resolved roll JSON + dev JSON + flags (unmatched gear, fuzzy dates, duplicate/odd frame numbers, missing camera prefix, unresolved focal→lens).

- [ ] **Step 3 — Review (USER):** Approve / correct the batch. Genuinely-missing gear → add to vocab + lookup (Task 1.5 pattern) before importing the roll.

- [ ] **Step 4 — Import (two-step per roll):** POST the roll, capture the returned **numeric id**, and if a `.dev.json` exists, inject that id and POST the self-dev record:

```bash
for f in ~/kammerz-import/staging/notes/<batch-glob>*.json; do
  [ "${f##*.dev.json}" ] || continue   # skip .dev.json files in this loop
  id=$(curl -sS -X POST http://localhost:3002/api/import/roll \
        -H 'Content-Type: application/json' --data-binary @"$f")
  echo "roll $(basename "$f"): id=$id"
  dev="${f%.json}.dev.json"
  if [ -f "$dev" ]; then
    python3 -c "import json,sys;d=json.load(open('$dev'));d['roll_id']=int('$id');print(json.dumps(d))" \
      | curl -sS -X POST http://localhost:3002/api/development/self \
          -H 'Content-Type: application/json' --data-binary @- ; echo " <- self-dev"
  fi
done
```

Expected: each roll returns a numeric id; developed rolls also create a self-dev record. A non-numeric `id` (e.g. an `{error}` body) means a 422 — investigate (duplicate `roll_id` is the resume backstop).

- [ ] **Step 5 — Mark done:** Append imported `roll_id`s to `~/kammerz-import/staging/notes-progress.md`. Update the bd epic notes with the running count (`<done>/<N>`).

- [ ] **Step 6 — Loop:** Repeat Steps 1–5 until every corpus note is imported or explicitly flagged-and-skipped. Periodically:

```bash
curl -fsS http://localhost:3002/api/rolls | python3 -c 'import sys,json;print("total rolls:",len(json.load(sys.stdin)))'
```

---

## Phase 3 — NotePlan disposition

### Task 3.1: Spot-check, then skip (default) or include

- [ ] **Step 1 (USER):** Pick 3–4 rolls that exist in both NotePlan and Apple Notes. Compare the NotePlan version against the Apple Notes original (paste both, or export the NotePlan note).

- [ ] **Step 2:** Decide:
  - **No added detail** (expected) → **skip NotePlan entirely.** Done. Don't add the NotePlan MCP server.
  - **Some rolls enriched** → export only the enriched NotePlan notes (markdown files or MCP), run them through the Phase 2b loop. The UNIQUE `roll_id` index makes re-imports of unchanged duplicates 422 harmlessly; for enriched ones, decide update-vs-skip per roll with the user (note: `import/roll` creates; updating an existing roll uses the rolls/shots PUT endpoints, not this importer).

---

## Phase 4 — Wrap-up & promotion

### Task 4.1: Final verification

- [ ] **Step 1:** Total count vs. expectation:

```bash
curl -fsS http://localhost:3002/api/rolls | python3 -c 'import sys,json;print("rolls:",len(json.load(sys.stdin)))'
curl -fsS http://localhost:3002/api/stats | python3 -m json.tool
```

Expected: ≈ (Notes N + Numbers 36 − duplicates). Spot-check 5 random rolls in the app against their source notes.

- [ ] **Step 2:** Skim for orphans/anomalies: rolls with no camera, impossible dates, empty shot lists where the note had shots. Fix via the app UI or re-POST.

### Task 4.2: Promote to the NAS (when satisfied)

- [ ] **Step 1 (USER):** The local dev DB now holds the full catalog. Promote it to the server per the project's deploy flow (copy the DB into the server's data dir, or re-run the import against the deployed instance). IDs travel with the DB file. Confirm with the user which promotion path they want — this is outside the importer's scope.

### Task 4.3: Clean up

- [ ] **Step 1:** Keep `~/kammerz-import/` until promotion is confirmed (it's the only copy of the staging/lookup data). Then archive or delete it.
- [ ] **Step 2:** Decide the fate of the `import-roll-data` worktree/branch with the user (it holds the tracked tooling + this plan; merge to main if you want the tooling kept, else `ExitWorktree remove`).
- [ ] **Step 3:** Update the bd epic to closed once the import is verified and promoted.

---

## Self-review notes

- **Spec coverage:** Phase 0 (export+recon) ✓, Phase 1 (vocabulary, roll-ID-prefix spine, catalog-note seeding, FK order, fixed-lens path) ✓, Phase 2a (Numbers dry-run) ✓, Phase 2b (Notes batch, confidence-tiered, beads) ✓, Phase 3 (NotePlan spot-check/skip) ✓, safety (DB backup, archived, review-before-write, idempotent) ✓, public-repo data split ✓.
- **Interactive-by-nature steps:** freeform parsing and gear matching can't be unit-tested; their "verification" is the staging-doc human review + round-trip `GET`/`curl` checks + the endpoint's own validation (422s). This is intentional, not a placeholder.
- **Field-name risk:** the API-reference table is illustrative; Task 0.6 Step 4 reconciles it against the live DTOs before any write.

```
```
