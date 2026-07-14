# Kammerz Manual Test Plan

## How to Use This Plan

- Run with `just dev` — backend (axum on `:3002`) + frontend (Vite on `:5273`, proxies `/api` → `:3002`). Open <http://localhost:5273>.
  - Alternatively, build the release binary (`just build`) and run it on `:3002`; it serves both the embedded SPA and the API at <http://localhost:3002>.
- Dev DB location: `./kammerz.db` (set by `DATABASE_URL`, default `sqlite:./kammerz.db?mode=rwc`). Migrations run + seed automatically on first launch.
- **Auth:** when `KAMMERZ_PASSWORD_HASH` is set, the app requires login (single shared password). When it is unset, auth is OPEN (LAN-trust mode) and a startup warning is logged — the login page and Sign-out control are effectively bypassed. Test the auth section against a build with the hash set.
- Use `sqlite3 ./kammerz.db` to verify data directly when needed.
- Mark each test: PASS / FAIL / SKIP (with reason)

---

## 0. Auth & Session (when `KAMMERZ_PASSWORD_HASH` is set)

### 0.1 Login

- [ ] Visiting any `(app)` route while unauthenticated redirects to `/login?next=<path>`
- [ ] Login page shows the password field; Enter submits (no separate form submit)
- [ ] Correct password logs in and redirects to the `next` path (or `/` if none)
- [ ] Incorrect password shows "Incorrect password" (no redirect)
- [ ] `?next=` only honors same-origin paths — a cross-origin/protocol-relative `next` falls back to `/`; `?next=/login` falls back to `/`

### 0.2 Logout & Rate Limiting

- [ ] Sidebar shows a **Sign out** control (only when auth is required)
- [ ] Sign out ends the session and redirects to `/login`; app routes then redirect back to login
- [ ] Rapid repeated bad-password attempts get throttled: ~5 attempts burst, then `429` (replenishes ~1 attempt / 10s) — error renders in the same `{error}` envelope as other failures
- [ ] In open mode (no hash set): no login wall, Sidebar **Sign out** is hidden, `GET /api/auth/me` reports `auth_required: false`

---

## 1. Dashboard (`/`)

### 1.1 Empty State (fresh DB)

- [ ] Shows catalog stat tiles (Total Rolls, Cameras, Lenses, Film Stocks)
- [ ] Shows empty-state CTA ("Start your log") with "Add Cameras" and "Create Roll" buttons that navigate correctly
- [ ] "+ New Roll" header button works

### 1.2 Populated State

- [ ] **In the Field** shows `loaded`/`shooting` rolls only
- [ ] **In the Darkroom** shows post-shooting rolls — `shot`, `at-lab`, `lab-done`, `developing`, `developed`, `scanned`, `post-processed` — sorted by pipeline order
- [ ] **Roll Pipeline** distribution bar shows colored segments per status (only statuses with count > 0); legend lists each with its count; hover title shows "Label: count"
- [ ] Quick-stats row shows Total Rolls, Cameras, In the Field, In the Darkroom counts
- [ ] **Needs Attention** shows rolls with no camera assigned (excluding `archived`)
- [ ] All non-archived rolls appear in at least one section
- [ ] Roll cards link to `/rolls/[id]?from=dashboard`; cards show roll_id, status badge, frame counter, camera + film stock, ISO

---

## 2. Cameras (`/cameras`)

### 2.1 List View

- [ ] All cameras load in card grid (`260px` min-width)
- [ ] Cards show brand, model, format, type, mount
- [ ] **Ownership tabs**: All / Owned / Sold filter correctly
  - Owned = `date_sold` is NULL
  - Sold = `date_sold` is set

### 2.2 Search & Organization (ListToolbar)

- [ ] Search filters by brand, model, type, serial number (case-insensitive)
- [ ] Group by: Brand, Mount, Format, Type, None
- [ ] Group headers show with ledger-line divider (hidden for "None")
- [ ] Sort: A–Z, Z–A, Newest Purchase, Oldest Purchase, Recently Added, Format

### 2.3 Create Camera (Standard)

- [ ] "Add Camera" opens dialog
- [ ] Required fields enforced: brand, model
- [ ] Format dropdown shows all options: 35mm, Medium Format (generic), Medium Format: 6x4.5 / 6x6 / 6x7 / 6x8 / 6x9, Large Format (generic), Large Format: 4x5 / 5x7 / 8x10, Instant
- [ ] Type dropdown: Not specified, SLR, Rangefinder, TLR, View Camera, Point & Shoot, Box Camera, Instant
- [ ] Mount dropdown filters/sorts by selected format family
- [ ] Brand autocomplete suggests existing brands (ComboInput)
- [ ] Purchased-from autocomplete suggests existing vendors
- [ ] Save creates camera, card appears in list

### 2.4 Create Camera (Fixed Lens)

- [ ] Selecting "Fixed Lens" mount transforms the form
- [ ] Inline lens fields appear: model, focal length, max aperture
- [ ] Save creates camera + lens + junction + sets default_lens_id (transactional)
- [ ] Camera detail shows "Built-in Lens" section
- [ ] Mount field is read-only on edit

### 2.5 Camera Detail (`/cameras/[id]`)

- [ ] All camera info displayed correctly
- [ ] **Compatible Lenses** section shows mount-compatible lenses
- [ ] "Link Lens" button opens picker with compatible lenses; Unlink (`×`) removes the junction (not available for fixed-lens)
- [ ] **Built-in Lens** section shown for fixed-lens cameras (no unlink/link/default-change controls)
- [ ] **Maintenance History** section: add/delete records; type options CLA, Repair, Cleaning, Modification, Other
- [ ] **Rolls (N)** section lists all rolls using this camera
- [ ] Edit locks the mount field to read-only
- [ ] Delete camera: confirmation dialog, then cascades

---

## 3. Lenses (`/lenses`)

### 3.1 List View

- [ ] Cards render in grid (`280px` min-width, wider for edit/delete buttons)
- [ ] Shows brand, model, mount, focal length, max aperture
- [ ] Ownership tabs: All / Owned / Sold
- [ ] Fixed-lens indicators: "Fixed on [Camera]" label on card

### 3.2 Search & Organization

- [ ] Search by brand, model, focal length, serial, lens system
- [ ] Group by: Brand, Mount, None
- [ ] Sort: A–Z, Z–A, Focal Length ↑, Focal Length ↓, Newest Purchase, Oldest Purchase, Recently Added

### 3.3 Create/Edit Lens

- [ ] Required fields: brand, mount
- [ ] Optional: model (brand-free; hint "Don't include the brand"), focal length, aperture range, filter threads (front/rear), serial, dates, notes
- [ ] `lensDisplayName` shows brand + model without doubling the brand
- [ ] Fixed-lens edit shows accent banner (read-only mount)

### 3.4 Delete Lens

- [ ] Confirmation dialog
- [ ] Cascades: camera_lenses, shot_lenses removed

---

## 4. Film Stocks (`/film-stocks`)

### 4.1 List View

- [ ] Rows show brand + name, ISO badge, format, type, exposure count
- [ ] **Type tabs**: All / Color / B&W / **Slide** — the Slide tab spans both `color-slide` and `bw-slide` stocks
- [ ] **Format tabs**: All Formats / 35mm / 120 / 4x5 / Instant
- [ ] ListToolbar search (brand, name, format, type); Group by Brand / Format / Type / None
- [ ] Sort: A–Z, Z–A, ISO Low–High, ISO High–Low, Recently Added, Format
- [ ] `?q=` in the URL pre-fills the search box (e.g. from a search-result link)

### 4.2 Create/Edit Film Stock

- [ ] "+ Add Film Stock" opens dialog; existing stocks open an **Edit** dialog (parallel fields)
- [ ] Required: brand, name (validated client-side with inline error)
- [ ] Format options: 135 / 35mm, 120, 4x5, 5x7, 8x10, Instant
- [ ] Type options: Color Negative, B&W Negative, Color Slide, B&W Slide
- [ ] **120 film**: leave exposure_count empty (frame count depends on back size — hint reads "Leave empty for variable (120 film)")
- [ ] **Sheet film** (4x5/5x7/8x10): exposure_count = 1 per sheet
- [ ] Cancel resets the form (no stale data)

---

## 5. Labs (`/labs`)

- [ ] List shows name, location, website
- [ ] Search by name, location
- [ ] Create: name required, location/website/notes optional
- [ ] Edit/Delete with confirmation

---

## 6. Rolls (`/rolls`)

### 6.1 List View

- [ ] Rows show roll_id (monospace), status badge, camera, film stock
- [ ] Status tabs filter: All, Loaded, Shooting, Shot, At Lab, Lab Done, Developing, Developed, Scanned, Post-processed, Archived
- [ ] Search by roll_id, camera, film stock, lens, notes
- [ ] Group by: None, Status, Camera, Film Stock
- [ ] Sort: Newest/Oldest Loaded, Newest/Oldest Finished, Roll ID A–Z/Z–A, Recently Added

### 6.2 Create Roll (`/rolls/new`)

- [ ] Roll ID auto-suggested (YYMMDD-N), editable
- [ ] Camera dropdown (excludes sold cameras); auto-disambiguates duplicates
- [ ] Film stock filtered/reordered by camera format (matching format first, "── Other formats ──" divider, then rest)
- [ ] Lens dropdown shows mount-compatible lenses (if camera selected)
- [ ] Frame count hint for 120/sheet film
- [ ] Default status: "loaded"
- [ ] Save redirects to roll detail page

### 6.3 Roll Detail (`/rolls/[id]`)

#### View & Edit

- [ ] All roll info displayed: roll_id, status, camera, film, lens, ISO, push/pull, frame count, dates, notes
- [ ] Frame progress: FrameCounter + "X/Y" bar (turns red when shots exceed frame count)
- [ ] Edit mode toggles a form with current values
- [ ] Film stock dropdown re-filters when camera changes; default lens re-seeds on camera change
- [ ] Fixed-lens camera: default lens shown read-only in edit (persisted, not a stale id)
- [ ] Update saves, Cancel discards changes
- [ ] Push/Pull options: Normal (box speed), Pull -2, Pull -1, Push +1, Push +2, Push +3

#### Status Chevron Bar (path-aware)

- [ ] **Lab flow**: loaded → shooting → shot → at-lab → lab-done → scanned → post-processed → archived
- [ ] **Self flow**: loaded → shooting → shot → developing → developed → scanned → post-processed → archived
- [ ] **Undecided flow** (no dev record): loaded → shooting → shot → **Develop ⌄** → scanned → post-processed → archived
- [ ] Path label ("Lab Development" / "Self Development") shows above the bar once a path is determined; absent while undecided
- [ ] Chevron styles: current `bg-accent`; past `bg-surface-overlay` text-accent; future `bg-surface-raised` text-muted
- [ ] Each chevron has a tooltip/aria-label describing exactly what clicking it does (move forward / move back & confirm / record development / asks for a date)
- [ ] **Status help** "?" popover next to the "Status" header explains the click behaviors; click-away closes it
- [ ] **Develop ⌄** menu (undecided flow, at the `shot` step) offers "Lab" and "Self / Home"; choosing one opens the matching dev dialog and re-renders the bar to that flow
- [ ] Forward click into a date-bearing status whose date is unset → date prompt (defaults to today); confirming writes the date alongside the status
- [ ] Forward click into `at-lab`/`lab-done`/`developing`/`developed` with no dev record → opens the matching dev dialog (seeded so a "Lab Done"/"Developed" click lands at the right rung)
- [ ] Backward click → "Move Status Back" ConfirmDialog (never writes a date)
- [ ] Cancelling a prompt-opened dev dialog shows an inline "Status unchanged" notice (no silent drop)

#### Timeline

- [ ] Read-only lifecycle dates render in path-aware order
- [ ] Inline date edit on a milestone persists to the owning record (roll / lab dev / self dev) and refreshes

#### Shots Section

- [ ] Lists all shots with frame#, aperture, shutter, lens, date, location, notes
- [ ] **Add Shot**: frame# required (inline error if blank), auto-suggested next frame
- [ ] **Smart defaults**:
  - Date: last shot's date > roll.date_loaded > empty
  - Lens: fixed lens (locked) > last-used on roll > roll.lens_id > camera.default_lens_id > empty
- [ ] Invalid date blocks Save / Save & Next
- [ ] **Save & Next** (add mode only): keeps dialog open, resets aperture/shutter/notes, preserves date/location/lens, auto-suggests next frame number
- [ ] Edit shot: form pre-filled, lens reassignment works
- [ ] Delete shot: confirmation, cascades shot_lenses
- [ ] "Quick Entry" shortcut button links to `/quick-entry?roll=<id>`

#### Roll-Full Nudge

- [ ] When `shooting` and shots ≥ frame count: "Roll complete" banner with a "Finished shooting" date field → "Mark as Shot" (writes `date_finished`); dismissible
- [ ] When shots exceed frame count: amber warning about extra frames / counting error

#### Status Auto-Sync (backend-owned, transactional)

- [ ] First shot added: `loaded` → `shooting`
- [ ] All shots deleted: `shooting`/`shot` → `loaded`
- [ ] Lab dev created → `lab-done` if a `date_received` is recorded, else `at-lab`
- [ ] Self dev created → `developed` if a `date_processed` is recorded, else `developing`
- [ ] Lab dev edited: clearing `date_received` reverts `lab-done` → `at-lab`; adding it advances; a roll already past (e.g. `scanned`) is untouched
- [ ] Self dev edited: clearing `date_processed` reverts `developed` → `developing`; symmetric to lab
- [ ] **Cross-flow adoption on create**: a roll orphaned on the self path (`developing`/`developed`) with no self record adopts onto the lab path when a lab dev is created (and vice-versa) — but only when no sibling record exists; a surviving sibling record keeps its status
- [ ] Lab dev deleted (no self dev): `at-lab`/`lab-done` → `shot`
- [ ] Self dev deleted (no lab dev): `developing`/`developed` → `shot`
- [ ] Status beyond a record's range is unaffected (e.g., deleting a dev at `scanned`/`post-processed` leaves it there)
- [ ] No frontend `$effect` drives status — only the backend changes it (UI refreshes after the call)

#### Development Records

- [ ] **Lab Dev**: lab dropdown, date dropped off, date received, cost, notes
  - Auto-opens when forwarding to a lab status with no record
  - One per roll (unique constraint)
  - Delete reverts status if applicable
- [ ] **Self Dev**: developer + dilution, stop bath, fixer + dilution, temperature, agitation notes, stages, notes
  - Auto-opens when forwarding to a self status with no record
  - One per roll
  - **Stages**: name, duration (MM:SS), sort order; add/remove inline
  - Delete reverts status if applicable
- [ ] Lab and self dev mutually exclusive in the UI (the "Develop" picker / dialogs gate which one can be created)

#### Back Navigation

- [ ] `?from=dashboard` → back to `/` ("Dashboard")
- [ ] `?from=developments` → back to `/developments` ("Developing")
- [ ] `?from=search` → back to `/search` ("Search")
- [ ] No `from` param → back to `/rolls` ("Rolls")

### 6.4 Delete Roll

- [ ] Confirmation dialog ("Permanently delete roll … This cannot be undone.")
- [ ] Cascades: shots, shot_lenses, lab dev, self dev, dev stages

---

## 7. Quick Entry (`/quick-entry`)

- [ ] Roll dropdown: active rolls (`loaded`/`shooting`/`shot`) first, then an "── Other rolls ──" divider; each option shows roll_id, film stock, and the **human status label** (e.g. "At Lab", not "at-lab")
- [ ] `?roll=<id>` pre-selects a roll
- [ ] Roll info bar shows camera, film stock, ISO, frame progress
- [ ] Frame number auto-suggested, required
- [ ] Aperture + shutter persist across saves in session; lens persists; notes clear after save
- [ ] Lens dropdown shows mount-compatible lenses for the selected roll's camera; fixed-lens camera locks the lens field
- [ ] "Save & Next →" creates the shot, increments frame, shows a success flash, increments session count, refocuses aperture
- [ ] ⌘/Ctrl+Enter saves
- [ ] **Roll-full nudge**: when the roll fills up while `shooting`, a "Roll complete" banner offers a "Finished shooting" date → "Mark as Shot" (writes `date_finished`); dismissible
- [ ] Previous Shots list shows the last 10 (newest first), most-recent animates in

---

## 8. Developments (`/developments`)

- [ ] **Self / Lab tabs** switch the dataset; switching resets the toolbar to that tab's defaults
- [ ] **Self** rows: developer + dilution, temperature, fixer/stop, stages (numbered, MM:SS), notes; trailing `date_processed`
- [ ] **Lab** rows: lab name, cost, dropped-off / received dates, notes
- [ ] Both rows show film stock, roll_id, status badge, and link to `/rolls/[id]?from=developments`
- [ ] Self search: developer, dilution, fixer, stop, temp, agitation, film stock, camera, roll_id, notes
- [ ] Lab search: lab, film stock, camera, roll_id, notes
- [ ] Self group: Developer / Film Stock / None; Self sort: Newest/Oldest Processed, Recently Added, Developer A–Z
- [ ] Lab group: Lab / Film Stock / None; Lab sort: Newest/Oldest Activity, Recently Added, Lab A–Z
- [ ] Empty states differ per tab (No Self-Developments / No Lab Developments)

---

## 9. Search (`/search`)

- [ ] Input autofocused, minimum 2 characters to trigger
- [ ] Debounced (no excessive API calls)
- [ ] Returns results across six types: cameras, lenses, film stocks, rolls, shots, labs
- [ ] Results grouped by entity type with an icon and a count
- [ ] Clicking a result navigates to its detail page with `?from=search`

---

## 10. Statistics (`/stats`)

- [ ] Summary tiles: Total Rolls, Total Shots, Total Costs
- [ ] **Cost Breakdown** section (lab dev / maintenance / combined)
- [ ] Rolls-per-month timeline
- [ ] **Top Film Stocks**, **Top Cameras**, **Top Lenses** by usage
- [ ] Distribution: **Rolls by Format**, **Rolls by Lens Mount**, **Rolls by Status** (status colors via `status.ts`)

---

## 11. Import Notes (`/import`)

### 11.1 Settings

- [ ] API-key field is write-only: saving clears it; a saved key shows "Key saved — enter a new key to replace" and the backend never returns the cleartext (masked sentinel)
- [ ] Eye toggle shows/hides the typed key
- [ ] Model dropdown auto-persists the selection; "Refresh" re-fetches the model list from the API (disabled until a key is saved)

### 11.2 Parse & Preview (3-step: input → preview → importing)

- [ ] "Parse with AI" requires a usable key (prompts to configure otherwise)
- [ ] Parsed data populates the preview: roll info + editable shots table
- [ ] Status dropdown includes all statuses through Post-processed and Archived (default Archived)
- [ ] Unmatched camera/film/lens show an amber warning with the AI's guess and a link to add the missing record
- [ ] Auto-match: camera by prefix, film stock / lens by fuzzy name match
- [ ] Shots are editable inline; rows removable
- [ ] **Date validation gate**: an invalid roll date or any invalid shot date (plain-text `YYYY-MM-DD` inputs) disables "Import" and shows the error; field border turns red
- [ ] "Import Roll & N Shots" creates roll + shots transactionally, then redirects to the new roll detail
- [ ] A failed import returns to the preview step with the error shown

---

## 12. Cross-Cutting Concerns

### Error Handling

- [ ] Required-field validation shows inline error text before the API call
- [ ] Backend constraint errors map to friendly messages (`friendly_err`)
- [ ] Duplicate roll_id: friendly "already exists" message
- [ ] Deleting a missing resource returns `404` (not a silent success)

### Destructive Actions

- [ ] Every delete has a ConfirmDialog
- [ ] No silent data deletion

### Dialog Behavior

- [ ] Cancel calls `resetForm()` (clears stale data)
- [ ] Scrollable content with `max-h-[85vh]`
- [ ] ComboInput dropdown uses `onmousedown` (not `onclick`) and is keyboard-accessible

### Animations

- [ ] FadeIn with staggered delays on page sections
- [ ] Dialogs render correctly after FadeIn animation ends (no z-index trapping)

### Visual Consistency

- [ ] All colors use CSS custom properties (no raw hex)
- [ ] Status badges always use the `<Badge>` component
- [ ] Card hover: `hover:border-accent/40`
- [ ] Section headers: `text-xs font-semibold uppercase tracking-wider text-text-faint`
- [ ] Select height matches `Input` (`h-[38px]`); native `<Input type="date">` and `TimeInput` fields align in the same row

---

## 13. End-to-End Workflows

### E2E-1: Full Roll Lifecycle (Lab Path)

1. [ ] Create a camera (with mount, format)
2. [ ] Create a film stock matching camera format
3. [ ] Create a roll linking camera + film stock
4. [ ] Add 3+ shots with different lenses → status auto-advances to `shooting`
5. [ ] Click chevron to `shot` (confirm finish-date nudge if the roll filled up)
6. [ ] Click chevron to `at-lab` → lab dev dialog auto-opens
7. [ ] Fill in lab dev (lab, dropped-off date, cost) → save → status syncs to `at-lab`
8. [ ] Add a `date_received` (or click "Lab Done") → status reaches `lab-done`
9. [ ] Advance to `scanned`, `post-processed`, `archived`
10. [ ] Verify the roll appears correctly in dashboard sections throughout
11. [ ] Delete the roll → verify all children cascade-deleted

### E2E-2: Full Roll Lifecycle (Self-Dev Path)

1. [ ] Use existing camera or create new
2. [ ] Create roll, add shots, advance to `shot`
3. [ ] Use the "Develop ⌄" menu → "Self / Home" → self dev dialog auto-opens
4. [ ] Fill self dev with developer, fixer, temp, and 4 stages (Dev, Stop, Fix, Rinse) → status syncs to `developing`
5. [ ] Add a `date_processed` (or click "Developed") → reaches `developed`
6. [ ] Advance to `scanned`, `post-processed`, `archived`
7. [ ] Verify the Developments page (Self tab) shows the record with stages

### E2E-3: Fixed-Lens Camera Flow

1. [ ] Create camera with "Fixed Lens" mount + inline lens fields
2. [ ] Verify camera detail shows "Built-in Lens" section
3. [ ] Verify lens list shows "Fixed on [Camera]" label
4. [ ] Create roll with this camera → lens dropdown locked
5. [ ] Add shots → lens pre-filled and read-only
6. [ ] Try Quick Entry with this roll → lens locked

### E2E-4: Status Revert & Adoption Scenarios

1. [ ] Create roll, add shots (status → `shooting`)
2. [ ] Delete all shots → status reverts to `loaded`
3. [ ] Add shots again, advance to `shot`, create lab dev (→ `at-lab`)
4. [ ] Delete lab dev → status reverts to `shot`
5. [ ] Create self dev (→ `developing`), advance to `scanned`
6. [ ] Delete self dev → status stays at `scanned` (beyond dev range)
7. [ ] (Cross-flow) Set a roll to `developing` with no self record, then create a **lab** dev → roll adopts the lab path at the data-driven target

### E2E-5: Quick Entry Session

1. [ ] Select an active roll
2. [ ] Enter 5 shots rapidly with "Save & Next →" (or ⌘/Ctrl+Enter)
3. [ ] Verify frame numbers auto-increment
4. [ ] Verify lens persists, notes clear, session count increments
5. [ ] Navigate to roll detail → verify all 5 shots present

### E2E-6: AI Import

1. [ ] Configure the API key + model in settings
2. [ ] Paste sample film notes → "Parse with AI"
3. [ ] Verify extracted roll + shots make sense; reconcile camera/film/lens (heed amber unmatched warnings)
4. [ ] Fix any invalid dates (import stays blocked until they're valid)
5. [ ] Import → verify roll + shots created and you land on the new roll detail

### E2E-7: Auth Round-Trip (hash set)

1. [ ] Visit `/rolls` while logged out → redirected to `/login?next=/rolls`
2. [ ] Log in → land back on `/rolls`
3. [ ] Sign out via the Sidebar → redirected to `/login`; app routes redirect back
4. [ ] Enter a wrong password ~6 times → throttled with a `429`

---

## 14. Database Integrity Checks

Run these with `sqlite3 ./kammerz.db` (or whatever `DATABASE_URL` points at):

```sql
-- Orphaned shots (no roll)
SELECT s.id FROM shots s LEFT JOIN rolls r ON s.roll_id = r.id WHERE r.id IS NULL;

-- Orphaned shot_lenses
SELECT sl.shot_id FROM shot_lenses sl LEFT JOIN shots s ON sl.shot_id = s.id WHERE s.id IS NULL;

-- Orphaned camera_lenses
SELECT cl.camera_id FROM camera_lenses cl LEFT JOIN cameras c ON cl.camera_id = c.id WHERE c.id IS NULL;

-- Rolls with invalid camera_id
SELECT r.id FROM rolls r LEFT JOIN cameras c ON r.camera_id = c.id WHERE r.camera_id IS NOT NULL AND c.id IS NULL;

-- Duplicate roll_ids
SELECT roll_id, COUNT(*) FROM rolls GROUP BY roll_id HAVING COUNT(*) > 1;

-- Multiple lab devs per roll (should be 0)
SELECT roll_id, COUNT(*) FROM development_labs GROUP BY roll_id HAVING COUNT(*) > 1;

-- Multiple self devs per roll (should be 0)
SELECT roll_id, COUNT(*) FROM development_selves GROUP BY roll_id HAVING COUNT(*) > 1;

-- Foreign keys enabled at runtime
PRAGMA foreign_keys;  -- Should return 1
```

> Note: a server-side backup snapshot is available at `GET /api/backup` (runs `VACUUM INTO` and returns the SQLite file). There is no UI for it — it's an API/CLI affordance for shell-based backup jobs.

---

## Appendix: Test Environment Setup

### Fresh Database

```bash
# Back up the existing dev DB
cp ./kammerz.db ./kammerz-backup.db

# Remove it (migrations recreate + seed on next launch)
rm ./kammerz.db ./kammerz.db-wal ./kammerz.db-shm 2>/dev/null

# Restart the app (migrations run automatically)
just dev
```

### Restore Database

```bash
cp ./kammerz-backup.db ./kammerz.db
```
