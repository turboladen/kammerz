# Kammerz Manual Test Plan

## How to Use This Plan

- Run with `bun run tauri dev`
- DB location: `~/Library/Application Support/com.kammerz.app/kammerz.db`
- Use `sqlite3` to verify data directly when needed
- Mark each test: PASS / FAIL / SKIP (with reason)

---

## 1. Dashboard (`/`)

### 1.1 Empty State (fresh DB)
- [ ] Shows empty state with Camera icon, "Start your log" text
- [ ] "Add Cameras" and "Create Roll" buttons are visible and navigate correctly

### 1.2 Populated State
- [ ] Status distribution bar shows colored segments per status
- [ ] Segment percentages add up to 100%
- [ ] **In the Field** shows `loaded`/`shooting` rolls only
- [ ] **In the Darkroom** shows `shot`/`at-lab`/`lab-done`/`developing`/`developed`/`scanned` rolls
- [ ] **Needs Attention** shows rolls with no camera assigned (excluding archived)
- [ ] All non-archived rolls appear in at least one section
- [ ] Roll cards link to `/rolls/[id]?from=dashboard`
- [ ] "New Roll" button works

---

## 2. Cameras (`/cameras`)

### 2.1 List View
- [ ] All cameras load in card grid (`260px` min-width)
- [ ] Cards show brand, model, format, type, mount
- [ ] **Ownership tabs**: All / Owned / Sold filter correctly
  - Owned = `date_sold` is NULL
  - Sold = `date_sold` is set

### 2.2 Search & Organization
- [ ] Search filters by brand, model, type, serial number (case-insensitive)
- [ ] Group by: Brand, Mount, Format, Type, None
- [ ] Group headers show with ledger-line divider (hidden for "None")
- [ ] Sort: A-Z, Z-A, Newest Purchase, Oldest Purchase, Recently Added, Format

### 2.3 Create Camera (Standard)
- [ ] "Add Camera" opens dialog
- [ ] Required fields enforced: brand, model
- [ ] Format dropdown shows all options (35mm through instant)
- [ ] Mount dropdown filters by selected format family
- [ ] Brand autocomplete suggests existing brands
- [ ] Purchased From autocomplete suggests existing vendors
- [ ] Save creates camera, card appears in list

### 2.4 Create Camera (Fixed Lens)
- [ ] Selecting "Fixed Lens" mount transforms the form
- [ ] Inline lens fields appear: model, focal length, max aperture
- [ ] Save creates camera + lens + junction + sets default_lens_id (transactional)
- [ ] Camera detail shows "Built-in Lens" section
- [ ] Mount field is read-only on edit

### 2.5 Camera Detail (`/cameras/[id]`)
- [ ] All camera info displayed correctly
- [ ] **Linked Lenses** section shows mount-compatible lenses
- [ ] "Link Lens" button opens picker with compatible lenses
- [ ] Unlink button removes junction (not available for fixed-lens)
- [ ] **Maintenance** section: add/edit/delete records (CLA, repair, cleaning, modification, other)
- [ ] **Rolls** section: lists all rolls using this camera
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
- [ ] Group by: Brand, Mount, Focal Length, Lens System, None

### 3.3 Create/Edit Lens
- [ ] Required fields: brand, mount
- [ ] Optional: model, focal length, aperture range, filter threads, serial, dates, notes
- [ ] Fixed-lens edit shows accent banner (read-only mount)

### 3.4 Delete Lens
- [ ] Confirmation dialog
- [ ] Cascades: camera_lenses, shot_lenses removed

---

## 4. Film Stocks (`/film-stocks`)

### 4.1 List View
- [ ] Cards show brand, name, format, ISO, stock type
- [ ] No owned/sold tabs (consumable item)
- [ ] Search by brand, name, ISO
- [ ] Group by: Brand, Format, Type, None

### 4.2 Create/Edit Film Stock
- [ ] Required: brand, name, format, stock type
- [ ] Format: 135, 120, 4x5, 5x7, 8x10, instant
- [ ] Stock type: color-negative, bw-negative, color-slide, bw-slide
- [ ] **120 film**: exposure_count left NULL (frame count depends on back size)
- [ ] **Sheet film** (4x5/5x7/8x10): exposure_count = 1
- [ ] Frame count hint displays correctly for 120 and sheet

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
- [ ] Status tabs filter: All, Loaded, Shooting, Shot, At Lab, Lab Done, Developing, Developed, Scanned, Archived
- [ ] Search by roll_id, camera, film stock, lens, notes
- [ ] Group by: None, Status, Camera, Film Stock
- [ ] Sort: Newest/Oldest Loaded/Finished, Roll ID A-Z/Z-A, Recently Added

### 6.2 Create Roll
- [ ] Roll ID auto-suggested, editable
- [ ] Camera dropdown (excludes sold cameras)
- [ ] Film stock filtered/reordered by camera format
- [ ] Lens dropdown shows mount-compatible lenses (if camera selected)
- [ ] Frame count hint for 120/sheet film
- [ ] Default status: "loaded"
- [ ] Save redirects to roll detail page

### 6.3 Roll Detail (`/rolls/[id]`)

#### View & Edit
- [ ] All roll info displayed: roll_id, status, camera, film, lens, dates, notes
- [ ] Frame progress bar: "Shot X of Y"
- [ ] Edit mode toggles form with current values
- [ ] Film stock dropdown re-filters when camera changes
- [ ] Update saves, Cancel discards changes

#### Status Chevrons
- [ ] **Lab flow**: loaded > shooting > shot > at-lab > lab-done > scanned > archived
- [ ] **Self flow**: loaded > shooting > shot > developing > developed > scanned > archived
- [ ] **Undecided flow**: loaded > shooting > shot > [gap] > scanned > archived
- [ ] Current chevron: `bg-accent`; past: `bg-accent/10`; future: `bg-surface-overlay`
- [ ] Forward click: instant status update (no confirmation)
- [ ] Backward click: ConfirmDialog before reverting

#### Shots Section
- [ ] Lists all shots with frame#, aperture, shutter, date, location, lens
- [ ] **Add Shot**: frame# required, auto-suggested next frame
- [ ] **Smart defaults**:
  - Date: last shot's date > roll.date_loaded > empty
  - Lens: fixed lens (locked) > last-used on roll > roll.lens_id > camera.default_lens_id > empty
- [ ] **Save & Next** (add mode only):
  - Keeps dialog open
  - Resets aperture, shutter, notes
  - Preserves date, location, lens
  - Auto-increments frame number
- [ ] Edit shot: form pre-filled, lens reassignment works
- [ ] Delete shot: confirmation, cascades shot_lenses

#### Status Auto-Sync
- [ ] First shot added: `loaded` -> `shooting`
- [ ] All shots deleted: `shooting`/`shot` -> `loaded`
- [ ] Lab dev created: -> `at-lab`
- [ ] Lab dev deleted (no self dev): `at-lab`/`lab-done` -> `shot`
- [ ] Self dev created: -> `developing`
- [ ] Self dev deleted (no lab dev): `developing`/`developed` -> `shot`
- [ ] Status beyond data range unaffected (e.g., deleting dev at `scanned` stays `scanned`)

#### Development Records
- [ ] **Lab Dev**: lab dropdown, date dropped/received, cost, notes
  - Auto-opens when status moved to "at-lab" (if no record exists)
  - One per roll (unique constraint)
  - Delete: auto-reverts status if applicable
- [ ] **Self Dev**: developer, dilution, fixer, temp, stages, notes
  - Auto-opens when status moved to "developing" (if no record exists)
  - One per roll
  - **Stages**: name, duration (MM:SS), notes, sort order
  - Add/remove stages inline
  - Delete: auto-reverts status if applicable
- [ ] Lab and self dev mutually exclusive UI: hides "+ Lab" / "+ Self" button once one exists

#### Back Navigation
- [ ] `?from=dashboard` -> back to `/`
- [ ] `?from=developments` -> back to `/developments`
- [ ] `?from=search` -> back to `/search`
- [ ] No `from` param -> back to `/rolls`

### 6.4 Delete Roll
- [ ] Confirmation dialog
- [ ] Cascades: shots, shot_lenses, lab dev, self dev, dev stages

---

## 7. Quick Entry (`/quick-entry`)

- [ ] Roll dropdown: active rolls (loaded/shooting/shot) appear first with status indicator
- [ ] Frame number auto-suggested, required
- [ ] Aperture and shutter speed persist across saves in session
- [ ] Lens dropdown shows mount-compatible lenses for selected roll's camera
- [ ] Fixed-lens camera: lens field locked
- [ ] "Save" creates shot, increments frame, clears notes, shows success count
- [ ] Date/lens persist as session defaults across saves

---

## 8. Developments (`/developments`)

- [ ] Lists all self-development records with roll/film/camera context
- [ ] Each row: roll_id, status, film stock, camera, developer+dilution, fixer, temp, stages
- [ ] Stage durations formatted as MM:SS
- [ ] Search by developer, dilution, fixer, temp, film stock, camera, roll_id
- [ ] Group by: Developer, Film Stock, None
- [ ] Sort: Newest/Oldest Processed, Recently Added, Developer A-Z

---

## 9. Search (`/search`)

- [ ] Input autofocused, minimum 2 characters to trigger
- [ ] Debounced (no excessive API calls)
- [ ] Returns results across: cameras, lenses, film stocks, rolls, shots, labs
- [ ] Each result shows: match_field, match_snippet with context
- [ ] Results grouped by entity type with icons
- [ ] Clicking result navigates to detail page with `?from=search`

---

## 10. Statistics (`/stats`)

- [ ] Total counts: rolls, shots, cameras, lenses
- [ ] Cost totals: lab dev, maintenance, combined
- [ ] Rolls per month timeline
- [ ] Top film stocks, cameras, lenses by usage
- [ ] Distribution: rolls by format, status, mount

---

## 11. Import (`/import`)

- [ ] Settings: Claude API key and model selection
- [ ] Paste notes, click "Parse with AI"
- [ ] Parsed data displayed: roll info + shots array
- [ ] Reconciliation: match camera/film/lens to existing records
- [ ] Manual edits before import
- [ ] "Import" creates roll + shots + lens links transactionally
- [ ] Success shows created roll ID

---

## 12. Cross-Cutting Concerns

### Error Handling
- [ ] Required field validation shows inline error text before invoke()
- [ ] Backend constraint errors map to friendly messages (friendly_err)
- [ ] Duplicate roll_id: "A roll with that roll id already exists"
- [ ] Missing required field: "[field] is required"

### Destructive Actions
- [ ] Every delete has a ConfirmDialog
- [ ] No silent data deletion

### Dialog Behavior
- [ ] Cancel calls resetForm() (clears stale data)
- [ ] Scrollable content with `max-h-[85vh]`
- [ ] ComboInput dropdown uses onmousedown (not onclick)

### Animations
- [ ] FadeIn with staggered delays on page sections
- [ ] Dialogs render correctly after FadeIn animation ends (no z-index trapping)

### Visual Consistency
- [ ] All colors use CSS custom properties (no raw hex)
- [ ] Status badges always use `<Badge>` component
- [ ] Card hover: `hover:border-accent/40`
- [ ] Section headers: `text-xs font-semibold uppercase tracking-wider text-text-faint`
- [ ] Select height matches Input/DateInput (`h-[38px]`)

---

## 13. End-to-End Workflows

### E2E-1: Full Roll Lifecycle (Lab Path)
1. [ ] Create a camera (with mount, format)
2. [ ] Create a film stock matching camera format
3. [ ] Create a roll linking camera + film stock
4. [ ] Add 3+ shots with different lenses -> status auto-advances to `shooting`
5. [ ] Click chevron to `shot`
6. [ ] Click chevron to `at-lab` -> lab dev dialog auto-opens
7. [ ] Fill in lab dev (lab, dates, cost) -> save
8. [ ] Advance to `lab-done`, then `scanned`, then `archived`
9. [ ] Verify roll appears correctly in dashboard sections throughout
10. [ ] Delete the roll -> verify all children cascade-deleted

### E2E-2: Full Roll Lifecycle (Self-Dev Path)
1. [ ] Use existing camera or create new
2. [ ] Create roll, add shots
3. [ ] Advance to `shot`, then `developing` -> self dev dialog auto-opens
4. [ ] Fill self dev with developer, fixer, temp, 4 stages (Dev, Stop, Fix, Rinse)
5. [ ] Advance to `developed`, `scanned`, `archived`
6. [ ] Verify developments page shows the record with stages

### E2E-3: Fixed-Lens Camera Flow
1. [ ] Create camera with "Fixed Lens" mount + inline lens fields
2. [ ] Verify camera detail shows "Built-in Lens" section
3. [ ] Verify lens list shows "Fixed on [Camera]" label
4. [ ] Create roll with this camera -> lens dropdown locked
5. [ ] Add shots -> lens pre-filled and read-only
6. [ ] Try Quick Entry with this roll -> lens locked

### E2E-4: Status Revert Scenarios
1. [ ] Create roll, add shots (status -> `shooting`)
2. [ ] Delete all shots -> verify status reverts to `loaded`
3. [ ] Add shots again, advance to `shot`, create lab dev (-> `at-lab`)
4. [ ] Delete lab dev -> verify status reverts to `shot`
5. [ ] Create self dev (-> `developing`), advance to `scanned`
6. [ ] Delete self dev -> verify status stays at `scanned` (beyond dev range)

### E2E-5: Quick Entry Session
1. [ ] Select an active roll
2. [ ] Enter 5 shots rapidly using Save button
3. [ ] Verify frame numbers auto-increment
4. [ ] Verify date/lens persist, notes clear
5. [ ] Verify session count displays correctly
6. [ ] Navigate to roll detail -> verify all 5 shots present

### E2E-6: AI Import
1. [ ] Set API key in settings
2. [ ] Paste sample film notes
3. [ ] Parse and verify extracted data makes sense
4. [ ] Reconcile: match camera, film stock, lens to existing records
5. [ ] Import -> verify roll + shots created
6. [ ] Navigate to imported roll detail -> verify all data correct

---

## 14. Database Integrity Checks

Run these with `sqlite3 ~/Library/Application\ Support/com.kammerz.app/kammerz.db`:

```sql
-- Orphaned shots (no roll)
SELECT s.id FROM shot s LEFT JOIN roll r ON s.roll_id = r.id WHERE r.id IS NULL;

-- Orphaned shot_lenses
SELECT sl.shot_id FROM shot_lenses sl LEFT JOIN shot s ON sl.shot_id = s.id WHERE s.id IS NULL;

-- Orphaned camera_lenses
SELECT cl.camera_id FROM camera_lenses cl LEFT JOIN camera c ON cl.camera_id = c.id WHERE c.id IS NULL;

-- Rolls with invalid camera_id
SELECT r.id FROM roll r LEFT JOIN camera c ON r.camera_id = c.id WHERE r.camera_id IS NOT NULL AND c.id IS NULL;

-- Duplicate roll_ids
SELECT roll_id, COUNT(*) FROM roll GROUP BY roll_id HAVING COUNT(*) > 1;

-- Multiple lab devs per roll (should be 0)
SELECT roll_id, COUNT(*) FROM development_lab GROUP BY roll_id HAVING COUNT(*) > 1;

-- Multiple self devs per roll (should be 0)
SELECT roll_id, COUNT(*) FROM development_self GROUP BY roll_id HAVING COUNT(*) > 1;

-- Foreign keys enabled
PRAGMA foreign_keys;  -- Should return 1
```

---

## Appendix: Test Environment Setup

### Fresh Database
```bash
# Back up existing DB
cp ~/Library/Application\ Support/com.kammerz.app/kammerz.db ~/Desktop/kammerz-backup.db

# Remove DB (migrations will recreate + seed on next launch)
rm ~/Library/Application\ Support/com.kammerz.app/kammerz.db

# Restart app
bun run tauri dev
```

### Restore Database
```bash
cp ~/Desktop/kammerz-backup.db ~/Library/Application\ Support/com.kammerz.app/kammerz.db
```
