<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import {
		APERTURE_SUGGESTIONS,
		SHUTTER_SUGGESTIONS,
		isRecognizedAperture,
		isRecognizedShutter,
		normalizeAperture,
		normalizeShutter
	} from '$lib/utils/exposure';
	import TimeInput from '$lib/components/ui/TimeInput.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import DateConfirm from '$lib/components/ui/DateConfirm.svelte';
	import DevelopmentSection from '$lib/components/rolls/DevelopmentSection.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import ActivityBoard from '$lib/components/rolls/ActivityBoard.svelte';
	import ArchiveDialog from '$lib/components/rolls/ArchiveDialog.svelte';
	import FrameStrip from '$lib/components/rolls/FrameStrip.svelte';
	import ShotsTable from '$lib/components/rolls/ShotsTable.svelte';
	import QuickAddBar from '$lib/components/rolls/QuickAddBar.svelte';
	import RollActivity from '$lib/components/rolls/RollActivity.svelte';
	import FilmStrip from '$lib/components/ui/FilmStrip.svelte';
	import FrameCounter from '$lib/components/ui/FrameCounter.svelte';
	import { getRollDetail, updateRoll, deleteRoll } from '$lib/api/rolls';
	import { logShot } from '$lib/utils/shot-entry';
	import { listCameras } from '$lib/api/cameras';
	import { listFilmStocks } from '$lib/api/film-stocks';
	import { listLenses } from '$lib/api/lenses';
	import { createShot, updateShot, deleteShot, suggestNextFrame } from '$lib/api/shots';
	import { listLabs } from '$lib/api/labs';
	import { lensDisplayName, buildLensOptions } from '$lib/utils/lens';
	import { buildFrameCells } from '$lib/utils/frames';
	import { formatShotRow, resolveShotLensName, type ShotLensDisplay } from '$lib/utils/shot-table';
	import { buildShotUpdatePayload, shotFormsEqual, type ShotFormFields } from '$lib/utils/shot-form';
	import { buildCameraLabels } from '$lib/utils/disambiguate';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import {
		rollPhase,
		lastShotSummary,
		activityLabel,
		isDatedKind,
		ROLL_DATE_FIELD,
		slotDateLabel,
		type DateSlot,
		type RollDateField,
		type ArchivePayload,
		type DevAutoPrompt
	} from '$lib/utils/activity-board';
	import { todayLocal, dateFieldError } from '$lib/utils/date';
	import { parseTime } from '$lib/utils/time';
	import type {
		RollWithDetails,
		RollInsert,
		Camera,
		FilmStock,
		Lens,
		Shot,
		Lab,
		DevelopmentLab,
		DevelopmentSelf,
		DevStage,
		PushPull,
		LensMount,
		RollEvent,
		ActivityKind
	} from '$lib/types';
	import { Trash2, Printer, ChevronLeft, ChevronRight } from 'lucide-svelte';

	const id = $derived(Number(page.params.id));

	// Back navigation: read ?from= param to determine where we came from
	const backRoutes: Record<string, { href: string; label: string }> = {
		developments: { href: '/developments', label: 'Developing' },
		search: { href: '/search', label: 'Search' },
		dashboard: { href: '/', label: 'Dashboard' }
	};
	const fromParam = $derived(page.url.searchParams.get('from'));
	const backNav = $derived(
		fromParam && backRoutes[fromParam] ? backRoutes[fromParam] : { href: '/rolls', label: 'Rolls' }
	);

	let roll: RollWithDetails | undefined = $state();
	let cameras: Camera[] = $state([]);
	let filmStocks: FilmStock[] = $state([]);
	let loading = $state(true);
	let showDeleteConfirm = $state(false);
	let error = $state('');

	// Roll edit mode (identity/config only — lifecycle dates now live on the board)
	let editingRoll = $state(false);
	let editRollId = $state('');
	let editCameraId = $state('');
	let editFilmStockId = $state('');
	let editLensId = $state('');
	let editFrameCount = $state('');
	let editPushPull = $state('');
	let editNotes = $state('');

	// Shot state
	let shots: Shot[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let shotLensMap: Record<number, number[]> = $state({});
	let lensMounts: LensMount[] = $state([]);
	let showShotDialog = $state(false);
	let editingShotId: number | null = $state(null);
	let deletingShotId: number | null = $state(null);
	// The shot dialog opens view-first everywhere (kammerz-4she): clicking a shot shows
	// it read-only with an Edit button. 'edit' is the existing form (also the add path).
	let shotDialogMode = $state<'view' | 'edit'>('view');

	// Shot form fields
	let shotFrameNumber = $state('');
	let shotAperture = $state('');
	let shotShutterSpeed = $state('');
	let shotDate = $state('');
	let shotTime = $state('');
	let shotLocation = $state('');
	let shotNotes = $state('');
	let shotLensId = $state('');
	let shotError = $state('');
	// Snapshot of the form as loaded when the Edit dialog opened — navigation
	// compares against it to decide whether it must save first (kammerz-11o3).
	let shotOpenSnapshot: ShotFormFields | null = $state(null);
	// Guards double-fired navigation while a nav-triggered save is in flight.
	let shotNavSaving = $state(false);

	function currentShotFormFields(): ShotFormFields {
		return {
			frameNumber: shotFrameNumber,
			aperture: shotAperture,
			shutterSpeed: shotShutterSpeed,
			date: shotDate,
			time: shotTime,
			location: shotLocation,
			notes: shotNotes,
			lensId: shotLensId
		};
	}
	const shotDateError = $derived(dateFieldError(shotDate));
	// Time to send: canonical 24h `HH:MM` when valid (e.g. "1430" → "14:30"), null when
	// blank/whitespace, else the trimmed raw so the backend 422 surfaces a mistyped time
	// instead of it being silently dropped. Trims so whitespace-only collapses to null.
	const shotTimePayload = $derived(parseTime(shotTime) || shotTime.trim() || null);

	// Development state (shared with DevelopmentSection component)
	let labs: Lab[] = $state([]);
	let labDev: DevelopmentLab | null = $state(null);
	let selfDev: DevelopmentSelf | null = $state(null);
	let devStages: DevStage[] = $state([]);
	let devAutoPrompt: DevAutoPrompt | null = $state(null);
	// Notice shown near the activity board when a prompt-opened dev dialog is
	// cancelled — so a dropped "Start development" click is acknowledged, not silent.
	let devNotice = $state('');

	// Activity journal events (populated from /detail after each load)
	let events: RollEvent[] = $state([]);

	// The server-derived activity view drives every lifecycle display. Never re-derive
	// from dates (ADR-0013) — read the backend's activities/badge/group_key/done.
	const activities = $derived(roll?.activities ?? []);
	const phase = $derived(roll ? rollPhase(roll) : 'shooting');
	const shootingActivity = $derived(activities.find((a) => a.kind === 'shooting') ?? null);

	// Board expand/collapse: collapsed by default in the shooting phase (the tail
	// activities are all not-started then), expanded otherwise. `null` follows the
	// default; true/false is a user override, reset on roll navigation.
	let boardExpandedOverride = $state<boolean | null>(null);
	const boardExpanded = $derived(boardExpandedOverride ?? phase !== 'shooting');

	// Frame progress
	const frameProgress = $derived.by(() => {
		if (!roll?.frame_count) return null;
		return { current: shots.length, total: roll.frame_count };
	});

	// Roll-full nudge state
	let rollFullDismissed = $state(false);
	// Finish date shown in the roll-complete nudge. Seeded once per roll from the
	// roll's existing date_finished (or today when unset) — see loadRollData. The
	// per-roll guard keeps same-roll mutation reloads from clobbering an in-progress
	// edit while still re-seeding on navigation to a different roll.
	let finishDate = $state(todayLocal());
	let finishDateSeededFor: number | null = $state(null);
	const finishDateError = $derived(dateFieldError(finishDate));
	// Nudge appears only while shooting is genuinely in progress (no finish date, no
	// dev record) and every frame is exposed — completing Shooting sets date_finished.
	const showRollFullNudge = $derived(
		phase === 'shooting' &&
			shootingActivity?.state === 'in_progress' &&
			frameProgress !== null &&
			shots.length >= frameProgress.total &&
			!rollFullDismissed
	);

	const cameraLabels = $derived(buildCameraLabels(cameras));
	const cameraOptions = $derived([
		{ value: '', label: 'Not assigned' },
		...cameras.map((c) => ({ value: String(c.id), label: cameraLabels.get(c.id) ?? `${c.brand} ${c.model}` }))
	]);

	const selectedCamera = $derived(roll?.camera_id ? (cameras.find((c) => c.id === roll?.camera_id) ?? null) : null);

	// Fixed-lens camera detection (based on saved roll camera)
	const isFixedLensCamera = $derived(
		selectedCamera ? lensMounts.some((m) => m.id === selectedCamera.lens_mount_id && m.name === 'Fixed Lens') : false
	);
	const fixedLens = $derived(
		isFixedLensCamera && selectedCamera?.default_lens_id
			? (allLenses.find((l) => l.id === selectedCamera.default_lens_id) ?? null)
			: null
	);

	// Edit-mode: camera selected in the edit form (for reactive film stock / lens filtering)
	const editSelectedCamera = $derived(
		editCameraId ? (cameras.find((c) => c.id === Number(editCameraId)) ?? null) : null
	);

	// Edit-mode: fixed-lens detection for the camera selected in the edit form
	const editIsFixedLens = $derived(
		editSelectedCamera
			? lensMounts.some((m) => m.id === editSelectedCamera.lens_mount_id && m.name === 'Fixed Lens')
			: false
	);
	const editFixedLens = $derived(
		editIsFixedLens && editSelectedCamera?.default_lens_id
			? (allLenses.find((l) => l.id === editSelectedCamera.default_lens_id) ?? null)
			: null
	);

	// Camera format → film stock format mapping
	const cameraFormatToStockFormat: Record<string, string> = {
		'35mm': '135',
		'medium format': '120',
		'6x4.5': '120',
		'6x6': '120',
		'6x7': '120',
		'6x8': '120',
		'6x9': '120',
		'large format': '4x5',
		'4x5': '4x5',
		'5x7': '5x7',
		'8x10': '8x10',
		instant: 'instant'
	};

	function stockLabel(s: FilmStock): string {
		return `${s.brand} ${s.name} (${s.format}${s.iso ? ', ISO ' + s.iso : ''})`;
	}

	const editFilmStockOptions = $derived.by(() => {
		const matchingFormat = editSelectedCamera ? cameraFormatToStockFormat[editSelectedCamera.format] : null;
		if (!matchingFormat) {
			return [
				{ value: '', label: 'Not assigned' },
				...filmStocks.map((s) => ({ value: String(s.id), label: stockLabel(s) }))
			];
		}
		const matching = filmStocks.filter((s) => s.format === matchingFormat);
		const rest = filmStocks.filter((s) => s.format !== matchingFormat);
		const options: { value: string; label: string; disabled?: boolean }[] = [{ value: '', label: 'Not assigned' }];
		for (const s of matching) options.push({ value: String(s.id), label: stockLabel(s) });
		if (matching.length > 0 && rest.length > 0) {
			options.push({ value: '__divider__', label: '── Other formats ──', disabled: true });
		}
		for (const s of rest) options.push({ value: String(s.id), label: stockLabel(s) });
		return options;
	});

	const editLensOptions = $derived(buildLensOptions(allLenses, editSelectedCamera, 'No default lens', lensMounts));

	// Hint for medium format variable-back cameras and large format sheet film
	const editFrameCountHint = $derived.by(() => {
		const matchingFormat = editSelectedCamera ? cameraFormatToStockFormat[editSelectedCamera.format] : null;
		if (matchingFormat === '120' && !editFrameCount) {
			return '120 film: 6×4.5=15 · 6×6=12 · 6×7=10 · 6×8=9 · 6×9=8';
		}
		if (['4x5', '5x7', '8x10'].includes(matchingFormat ?? '') && !editFrameCount) {
			return 'Sheet film: total sheets loaded (e.g. 6 holders × 2 = 12)';
		}
		return undefined;
	});

	// Shot-level lens dropdown options (uses the saved camera, not the edit form camera)
	const shotLensOptions = $derived(buildLensOptions(allLenses, selectedCamera, 'No lens', lensMounts));

	// Per-shot lens display for the table + view dialog: shot's own lens > roll default.
	// The id→lens index is built once per catalog change so resolving every shot
	// stays O(shots), not O(shots × lenses).
	const lensById = $derived(new Map(allLenses.map((l) => [l.id, l])));
	const shotLenses = $derived.by(() => {
		const map: Record<number, ShotLensDisplay> = {};
		for (const s of shots) map[s.id] = resolveShotLensName(s.id, shotLensMap, lensById, roll?.lens_id ?? null);
		return map;
	});
	// The table renders the EFFECTIVE name only (it's what took the shot — the
	// transcription-correct value); provenance annotation lives in the view dialog.
	const shotLensNames = $derived.by(() => {
		const map: Record<number, string> = {};
		for (const [id, lens] of Object.entries(shotLenses)) map[Number(id)] = lens.name;
		return map;
	});

	// "What settings did I just use?" reference card for the shooting phase.
	const lastShot = $derived(lastShotSummary(shots, shotLensMap, allLenses));

	// Frame cells: map shots onto numbered slots, extras appended after (shared util).
	const frameCells = $derived(buildFrameCells(shots, roll?.frame_count ?? null));
	const nextFrameNumber = $derived(frameCells.find((c) => c.isNext)?.frameNumber ?? '');

	// Shots in FrameStrip order (skips empty slots) — for in-dialog prev/next nav.
	const orderedShots = $derived(frameCells.filter((c) => c.shot).map((c) => c.shot!));
	// The shot the dialog currently targets — drives the read-only view rendering. The
	// edit form is populated in parallel (populateShotForm) so switching to Edit is instant.
	const editingShot = $derived(editingShotId == null ? null : (shots.find((s) => s.id === editingShotId) ?? null));
	const editingShotRow = $derived(editingShot ? formatShotRow(editingShot) : null);

	// The view shows the EFFECTIVE lens (per-shot, else roll default). When it's the
	// inherited default, say so — the edit form's Lens dropdown deliberately shows
	// only the per-shot override ('No lens' when inheriting), and without the
	// annotation view→Edit reads like the lens was cleared. `inherited` comes from
	// resolveShotLensName itself, never re-derived from the raw lens map here.
	const viewLensLabel = $derived.by(() => {
		const lens = editingShot ? shotLenses[editingShot.id] : null;
		if (!lens || !lens.name) return '';
		return lens.inherited ? `${lens.name} (roll default)` : lens.name;
	});
	// True only while an existing shot is shown read-only (never in add mode).
	// Row presence is part of the guard: if the viewed shot vanishes from `shots`
	// (concurrent delete), the title/arrows/body must all agree instead of the body
	// silently falling through to the edit form under a view-mode header.
	const isShotView = $derived(editingShotId != null && shotDialogMode === 'view' && editingShotRow != null);
	const currentShotIdx = $derived(editingShotId == null ? -1 : orderedShots.findIndex((s) => s.id === editingShotId));
	const hasPrevShot = $derived(currentShotIdx > 0);
	const hasNextShot = $derived(currentShotIdx >= 0 && currentShotIdx < orderedShots.length - 1);

	// QuickAddBar save state
	let quickSaving = $state(false);
	let quickError = $state('');

	// QuickAddBar visibility: shown by default in the shooting phase, hidden in
	// wrap-up/done (post-shooting, transcribing metadata). null = follow the default;
	// true/false = user override. The Show/Hide entry toggle and FrameStrip open-slot
	// clicks still cover the forgotten-shot case in any phase.
	let quickAddOverride = $state<boolean | null>(null);
	const quickAddVisible = $derived(quickAddOverride ?? phase === 'shooting');

	// Frames view: the film strip (logging, frame-count progress) by default while
	// shooting; the shots table (zero-click reading for metadata transcription) in the
	// post-shooting phases. null = follow the default; 'strip'/'table' = user override,
	// reset on roll navigation like quickAddOverride/boardExpandedOverride.
	let shotViewOverride = $state<'strip' | 'table' | null>(null);
	const shotView = $derived(shotViewOverride ?? (phase === 'shooting' ? 'strip' : 'table'));

	// Default lens id for the QuickAddBar (mirrors the shot dialog's smart cascade,
	// minus the last-used-on-roll part which is session state — use roll/camera default)
	const quickDefaultLensId = $derived.by(() => {
		if (fixedLens) return String(fixedLens.id);
		if (roll?.lens_id) return String(roll.lens_id);
		if (selectedCamera?.default_lens_id) return String(selectedCamera.default_lens_id);
		return '';
	});

	// Reference catalogs (cameras, film stocks, lenses, labs, lens mounts) are
	// loaded by the page-load $effect — on mount and on roll-id navigation — but
	// NOT by the mutation refresh path (loadRollData), since no mutation on this
	// page can change them.
	async function loadRefData() {
		try {
			const [cams, stocks, lenses, labsList, mounts] = await Promise.all([
				listCameras(),
				listFilmStocks(),
				listLenses(),
				listLabs(),
				listLensMounts()
			]);
			cameras = cams;
			filmStocks = stocks;
			allLenses = lenses;
			labs = labsList;
			lensMounts = mounts;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function loadRollData() {
		// Clear any stale error/notice: this component instance is reused across [id]
		// changes (the $effect re-runs loadRollData()), so a prior failure must not
		// leak into the next roll's view.
		error = '';
		devNotice = '';
		try {
			// The composite /detail endpoint collapses the roll-scoped round-trips
			// (roll, shots, shot-lens pairs, lab/self dev, dev stages, events) into one.
			// Reference catalogs are loaded separately in loadRefData().
			const detail = await getRollDetail(id);
			roll = detail.roll;
			// Reset per-roll nudge state ONCE per roll: reflect an already-set
			// date_finished instead of misleadingly showing today, and clear a stale
			// roll-full dismissal — but NOT on same-roll mutation reloads, which would
			// clobber an in-progress finish-date edit or resurrect a dismissed banner.
			if (roll.id !== finishDateSeededFor) {
				finishDateSeededFor = roll.id;
				finishDate = roll.date_finished ?? todayLocal();
				rollFullDismissed = false;
			}
			shots = detail.shots;
			labDev = detail.lab_dev;
			selfDev = detail.self_dev;
			devStages = detail.dev_stages;
			events = detail.events ?? [];

			// Map the batched shot-lens associations by shot id
			const map: Record<number, number[]> = {};
			for (const [shotId, lensId] of detail.shot_lens_pairs) {
				(map[shotId] ??= []).push(lensId);
			}
			shotLensMap = map;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	function resetShotForm() {
		shotFrameNumber = '';
		shotAperture = '';
		shotShutterSpeed = '';
		shotDate = '';
		shotTime = '';
		shotLocation = '';
		shotNotes = '';
		shotLensId = '';
		shotError = '';
		shotOpenSnapshot = null;
	}

	async function openAddShotDialog(frame?: string) {
		resetShotForm();
		editingShotId = null;

		// Smart date default: last shot's date > roll's date_loaded > empty
		if (shots.length > 0) {
			const lastShotEntry = shots[shots.length - 1];
			if (lastShotEntry.date) shotDate = lastShotEntry.date;
		} else if (roll?.date_loaded) {
			shotDate = roll.date_loaded;
		}

		// Smart lens default: fixed lens > last-used on roll > roll default > camera default
		if (fixedLens) {
			shotLensId = String(fixedLens.id);
		} else if (shots.length > 0) {
			const lastShotEntry = shots[shots.length - 1];
			const lastLensIds = shotLensMap[lastShotEntry.id] ?? [];
			if (lastLensIds.length > 0) {
				shotLensId = String(lastLensIds[0]);
			} else if (roll?.lens_id) {
				shotLensId = String(roll.lens_id);
			} else if (selectedCamera?.default_lens_id) {
				shotLensId = String(selectedCamera.default_lens_id);
			}
		} else if (roll?.lens_id) {
			shotLensId = String(roll.lens_id);
		} else if (selectedCamera?.default_lens_id) {
			shotLensId = String(selectedCamera.default_lens_id);
		}

		// If a specific frame was requested (e.g. from FrameStrip open-slot click or ＋),
		// use it; otherwise fall back to suggestNextFrame.
		if (frame) {
			shotFrameNumber = frame;
		} else {
			try {
				shotFrameNumber = await suggestNextFrame(id);
			} catch {
				shotFrameNumber = '';
			}
		}
		showShotDialog = true;
	}

	// Seed the shared shot form from a shot and capture the dirty-compare baseline. Used
	// by both view and edit opens so switching view→Edit needs no reseed and the
	// kammerz-11o3 snapshot is already correct.
	function populateShotForm(shot: Shot) {
		editingShotId = shot.id;
		shotFrameNumber = shot.frame_number;
		shotAperture = shot.aperture ?? '';
		shotShutterSpeed = shot.shutter_speed ?? '';
		shotDate = shot.date ?? '';
		shotTime = shot.time ?? '';
		shotLocation = shot.location ?? '';
		shotNotes = shot.notes ?? '';
		const ids = shotLensMap[shot.id] ?? [];
		shotLensId = ids.length > 0 ? String(ids[0]) : '';
		shotError = '';
		shotOpenSnapshot = currentShotFormFields();
	}

	// Open the dialog read-only on a shot (view-first, kammerz-4she).
	function openShotView(shot: Shot) {
		populateShotForm(shot);
		shotDialogMode = 'view';
		showShotDialog = true;
	}

	// Open the dialog straight into the edit form (used by the view-mode Edit button
	// and prev/next auto-save navigation).
	function openShotEdit(shot: Shot) {
		populateShotForm(shot);
		shotDialogMode = 'edit';
		showShotDialog = true;
	}

	// Single teardown for every way the shot dialog closes — the mode reset had
	// already drifted across hand-inlined copies of this block.
	function closeShotDialog() {
		showShotDialog = false;
		resetShotForm();
		shotDialogMode = 'view';
	}

	// Close the dialog, then arm the (ConfirmDialog-gated) delete for its shot.
	function startDeleteShot() {
		const sid = editingShotId;
		closeShotDialog();
		deletingShotId = sid;
	}

	// Navigate to an adjacent shot, saving the current one's edits first if the
	// form is dirty (kammerz-11o3). The target is captured by POSITION before the
	// save/reload (that's the shot the user saw next to the arrow), then re-looked
	// up BY ID afterward — a frame-number edit can reorder orderedShots.
	async function navigateToShot(direction: -1 | 1) {
		if (shotNavSaving || editingShotId == null) return;
		const target = orderedShots[currentShotIdx + direction];
		if (!target) return;
		const fields = currentShotFormFields();
		if (shotOpenSnapshot == null || !shotFormsEqual(fields, shotOpenSnapshot)) {
			shotError = '';
			if (!fields.frameNumber.trim()) {
				shotError = 'Frame number is required.';
				return;
			}
			if (shotDateError) return; // arrows are disabled too — belt and suspenders
			shotNavSaving = true;
			try {
				await updateShot(editingShotId, buildShotUpdatePayload(fields));
				// loadRollData never throws — it catches internally and sets the page
				// `error` state — so check it explicitly: navigating on a failed reload
				// would seed the next shot from stale data.
				await loadRollData();
				if (error) return;
			} catch (err) {
				shotError = err instanceof Error ? err.message : String(err);
				return; // stay on this shot — the user can see and fix the failure
			} finally {
				shotNavSaving = false;
			}
		}
		const fresh = orderedShots.find((s) => s.id === target.id);
		if (fresh) openShotEdit(fresh);
	}

	// Prev/next branch on mode: edit mode auto-saves on navigate (kammerz-11o3); view
	// mode navigates freely with nothing to save, re-opening the adjacent shot read-only.
	function goPrevShot() {
		if (!hasPrevShot) return;
		if (shotDialogMode === 'view') {
			const target = orderedShots[currentShotIdx - 1];
			if (target) openShotView(target);
		} else {
			void navigateToShot(-1);
		}
	}
	function goNextShot() {
		if (!hasNextShot) return;
		if (shotDialogMode === 'view') {
			const target = orderedShots[currentShotIdx + 1];
			if (target) openShotView(target);
		} else {
			void navigateToShot(1);
		}
	}

	function handleShotDialogKeydown(e: KeyboardEvent) {
		if (!showShotDialog || !editingShotId) return;
		const target = e.target as HTMLElement;
		const tag = target?.tagName?.toLowerCase();
		if (tag === 'input' || tag === 'textarea' || tag === 'select') return;
		if (e.key === 'ArrowLeft') {
			e.preventDefault();
			goPrevShot();
		} else if (e.key === 'ArrowRight') {
			e.preventDefault();
			goNextShot();
		}
	}

	async function handleSaveShot() {
		shotError = '';
		if (!shotFrameNumber.trim()) {
			shotError = 'Frame number is required.';
			return;
		}
		const lensIds = shotLensId ? [Number(shotLensId)] : [];
		try {
			if (editingShotId) {
				await updateShot(editingShotId, buildShotUpdatePayload(currentShotFormFields()));
			} else {
				await createShot({
					roll_id: id,
					frame_number: shotFrameNumber.trim(),
					aperture: normalizeAperture(shotAperture) || null,
					shutter_speed: normalizeShutter(shotShutterSpeed) || null,
					date: shotDate || null,
					time: shotTimePayload,
					location: shotLocation || null,
					gps_lat: null,
					gps_lon: null,
					notes: shotNotes || null,
					lens_ids: lensIds
				});
			}
			closeShotDialog();
			await loadRollData();
		} catch (err) {
			shotError = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleSaveShotAndNext() {
		shotError = '';
		if (!shotFrameNumber.trim()) {
			shotError = 'Frame number is required.';
			return;
		}
		const lensIds = shotLensId ? [Number(shotLensId)] : [];
		const savedLensId = shotLensId;
		try {
			await createShot({
				roll_id: id,
				frame_number: shotFrameNumber.trim(),
				aperture: normalizeAperture(shotAperture) || null,
				shutter_speed: normalizeShutter(shotShutterSpeed) || null,
				date: shotDate || null,
				time: shotTimePayload,
				location: shotLocation || null,
				gps_lat: null,
				gps_lon: null,
				notes: shotNotes || null,
				lens_ids: lensIds
			});
			await loadRollData();
			// Reset per-shot fields but keep session defaults (date, time, location, lens)
			try {
				shotFrameNumber = await suggestNextFrame(id);
			} catch {
				shotFrameNumber = '';
			}
			shotAperture = '';
			shotShutterSpeed = '';
			shotNotes = '';
			shotLensId = savedLensId;
			shotError = '';
		} catch (err) {
			shotError = err instanceof Error ? err.message : String(err);
		}
	}

	async function confirmDeleteShot() {
		if (deletingShotId === null) return;
		const shotId = deletingShotId;
		// Close the dialog before the request — a failure is reported via the
		// page error banner, and the dialog stays re-openable.
		deletingShotId = null;
		error = '';
		try {
			await deleteShot(shotId);
			await loadRollData();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	// --- Activity board handlers ---

	// Shared PUT-then-reload for every board/nudge roll write. One copy of the
	// error-normalization + reload ordering so the flow can't drift per handler.
	async function patchRoll(patch: Partial<RollInsert>) {
		error = '';
		try {
			await updateRoll(id, patch);
			await loadRollData();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	// Editing (set/change) a roll-owned lifecycle date via DateConfirm.
	let dateEdit = $state<{ field: RollDateField; current: string | null; title: string } | null>(null);

	function onEditDate(kind: ActivityKind, slot: DateSlot) {
		if (!roll) return;
		const field = ROLL_DATE_FIELD[kind]?.[slot];
		if (!field) return;
		const current = (roll[field] ?? null) as string | null;
		// Title names the exact slot ("Set Shooting finished date") — the kind alone
		// is ambiguous between an activity's two slots, and nothing else in the
		// dialog says which column is being written. boardSlotLabel is the same
		// phrase source as the board's accessible names, so they can't drift.
		dateEdit = {
			field,
			current,
			title: `${current ? 'Edit' : 'Set'} ${boardSlotLabel(kind, slot)}`
		};
	}

	// One label producer for BOTH dialog flows (edit title, clear confirm) and the
	// clear path's focus-restore selector — the dated-kind phrase comes from
	// slotDateLabel, the non-dated fallback lives only here.
	function boardSlotLabel(kind: ActivityKind, slot: DateSlot): string {
		return isDatedKind(kind) ? slotDateLabel(kind, slot) : `${activityLabel(kind)} date`;
	}

	async function confirmDateEdit(date: string | null) {
		const edit = dateEdit;
		dateEdit = null;
		// DateConfirm here disallows clear (allowClear defaults false), so `date` is
		// always a value — a clear goes through the × control / onClearDate instead.
		if (!edit || date == null) return;
		await patchRoll({ [edit.field]: date } as Partial<RollInsert>);
	}

	// Clearing a set lifecycle date is a backward move — confirm first (ADR-0013).
	let pendingClear = $state<{ field: RollDateField; label: string } | null>(null);

	function onClearDate(kind: ActivityKind, slot: DateSlot) {
		const field = ROLL_DATE_FIELD[kind]?.[slot];
		if (!field) return;
		// Slot-qualified for the same reason as the edit title: "Clear the Shooting
		// date?" can't tell the Loaded × from the adjacent Finished ×.
		pendingClear = { field, label: boardSlotLabel(kind, slot) };
	}

	async function confirmClearDate() {
		const p = pendingClear;
		pendingClear = null;
		if (!p) return;
		// Pin the board open BEFORE the write: the × is only reachable on an
		// expanded board, but a clear that reverts the roll to the shooting phase
		// (e.g. un-finishing shooting) would flip the collapse default and remove
		// the very Set button the focus hand-off below targets.
		boardExpandedOverride = true;
		await patchRoll({ [p.field]: null } as Partial<RollInsert>);
		// The × that opened the confirm no longer exists (its date is gone), so the
		// dialog's focus restore lands on <body>. Hand focus to the slot's Set
		// button — the control that replaced it. No-ops if the label isn't found.
		await tick();
		document.querySelector<HTMLButtonElement>(`[aria-label="Set ${p.label}"]`)?.focus();
	}

	// Open the dev dialog via the auto-prompt bridge. Single entry point so every
	// opener (board Start buttons, board Edit, journal event click) clears a stale
	// "No development record was saved" notice before the dialog shows.
	function openDev(kind: 'lab' | 'self') {
		devNotice = '';
		devAutoPrompt = { kind };
	}

	// --- Archiving ---
	let showArchiveDialog = $state(false);
	// A save that would clear an already-set archive date is a backward move — hold
	// it for confirmation instead of applying it immediately.
	let pendingArchiveClear = $state<ArchivePayload | null>(null);

	function handleArchiveSave(payload: ArchivePayload) {
		showArchiveDialog = false;
		if (roll?.date_archived && payload.date_archived == null) {
			pendingArchiveClear = payload;
		} else {
			void patchRoll(payload);
		}
	}

	// Roll-full nudge: completing Shooting records date_finished (ADR-0013).
	async function markAsShot() {
		await patchRoll({ date_finished: finishDate });
	}

	const pushPullOptions = [
		{ value: '', label: 'Normal (box speed)' },
		{ value: '-2', label: 'Pull -2' },
		{ value: '-1', label: 'Pull -1' },
		{ value: '+1', label: 'Push +1' },
		{ value: '+2', label: 'Push +2' },
		{ value: '+3', label: 'Push +3' }
	];

	function startEditRoll() {
		if (!roll) return;
		editRollId = roll.roll_id;
		editCameraId = roll.camera_id?.toString() ?? '';
		editFilmStockId = roll.film_stock_id?.toString() ?? '';
		editLensId = roll.lens_id?.toString() ?? '';
		editFrameCount = roll.frame_count?.toString() ?? '';
		editPushPull = roll.push_pull ?? '';
		editNotes = roll.notes ?? '';
		editingRoll = true;
	}

	// Re-seed the default lens when the camera changes in edit mode (mirrors the
	// auto-populate effect in rolls/new). Without this, switching cameras would
	// silently keep a lens that can't mount on the new camera (kammerz-8hg).
	function handleEditCameraChange() {
		if (roll && editCameraId === (roll.camera_id?.toString() ?? '')) {
			// Back to the saved camera — restore the roll's saved lens
			editLensId = roll.lens_id?.toString() ?? '';
		} else {
			editLensId = editSelectedCamera?.default_lens_id?.toString() ?? '';
		}
	}

	async function saveEditRoll() {
		error = '';
		if (!editRollId.trim()) {
			error = 'Roll ID is required.';
			return;
		}
		try {
			// For fixed-lens cameras the lens Select is hidden and replaced by a
			// read-only display of the built-in lens — persist that lens, never a
			// stale editLensId left over from a previous camera (kammerz-8hg).
			const lensId = editIsFixedLens ? (editFixedLens?.id ?? null) : editLensId ? Number(editLensId) : null;
			await updateRoll(id, {
				roll_id: editRollId.trim(),
				camera_id: editCameraId ? Number(editCameraId) : null,
				film_stock_id: editFilmStockId ? Number(editFilmStockId) : null,
				lens_id: lensId,
				frame_count: editFrameCount ? parseInt(editFrameCount) : null,
				push_pull: (editPushPull || null) as PushPull | null,
				notes: editNotes || null
			});
			editingRoll = false;
			await loadRollData();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	function handleDelete() {
		showDeleteConfirm = true;
	}

	async function confirmDelete() {
		// ConfirmDialog no longer closes itself on confirm — the parent owns
		// closing, so reset the bound state before the request.
		showDeleteConfirm = false;
		error = '';
		try {
			await deleteRoll(id);
			goto('/rolls');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	// Open the dev dialog from an activity-journal event click. Reuses the autoPrompt
	// bridge: DevelopmentSection seeds its form from the existing record when present.
	function openDevFromEvent(refKind: 'lab_dev' | 'self_dev') {
		openDev(refKind === 'lab_dev' ? 'lab' : 'self');
	}

	// QuickAddBar → logShot → reload
	async function handleQuickAdd(entry: {
		frameNumber: string;
		aperture: string;
		shutterSpeed: string;
		lensId: string;
		date: string;
		time: string;
		location: string;
		notes: string;
	}): Promise<boolean> {
		if (!roll || !entry.frameNumber.trim()) return false;
		quickError = '';
		quickSaving = true;
		try {
			await logShot({ rollId: roll.id, ...entry });
			await loadRollData();
			return true;
		} catch (err) {
			quickError = err instanceof Error ? err.message : String(err);
			return false;
		} finally {
			quickSaving = false;
		}
	}

	// FrameStrip / table selection: filled frame → view-first dialog; open slot → add
	// dialog pre-seeded.
	function handleFrameSelect(frameNumber: string, shot: Shot | null) {
		if (shot) {
			openShotView(shot);
		} else {
			openAddShotDialog(frameNumber);
		}
	}

	// Page load: on mount and whenever the roll id changes (navigation), fetch
	// reference catalogs and roll detail together, gating `loading` until BOTH
	// resolve. Mutations bypass this and call loadRollData() directly.
	//
	// Track ONLY `id` (the `void id` read), then untrack the loaders: Svelte 5
	// tracks every reactive read in an effect's synchronous run — including the
	// sync prefix of an async fn it calls. loadRollData()'s prefix rewrites `roll`,
	// so without untrack the effect would loop on every roll-detail visit (kammerz-8k5).
	$effect(() => {
		void id;
		untrack(() => {
			// Re-apply per-roll UI defaults (quick-entry + board expansion follow the
			// phase) on every roll change so a manual override doesn't leak across rolls.
			quickAddOverride = null;
			boardExpandedOverride = null;
			shotViewOverride = null;
			// Close any board dialog state left from the previous roll: ArchiveDialog's
			// seed runs only on the open transition (untracked), so a dialog surviving
			// a roll change would hold — and then save — the PRIOR roll's draft.
			showArchiveDialog = false;
			dateEdit = null;
			pendingClear = null;
			pendingArchiveClear = null;
			loading = true;
			Promise.all([loadRefData(), loadRollData()]).finally(() => {
				loading = false;
			});
		});
	});
</script>

{#if loading}
	<PageHeader title="Loading..." />
{:else if !roll}
	<PageHeader title="Roll not found" />
	<div class="p-6">
		<Button href="/rolls">&larr; Back to rolls</Button>
	</div>
{:else}
	<PageHeader title="Roll {roll.roll_id}" backHref={backNav.href} backLabel={backNav.label}>
		<Button variant="ghost" href="/rolls/{roll.id}/print"
			><Printer size={16} strokeWidth={2} aria-hidden="true" />Print summary</Button
		>
		<Button variant="danger" onclick={handleDelete}
			><Trash2 size={16} strokeWidth={2} aria-hidden="true" />Delete</Button
		>
	</PageHeader>

	<div class="p-6">
		{#if error}
			<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}

		<!-- Roll Header -->
		<FadeIn delay={0}>
			<div class="relative mb-6 overflow-hidden rounded-lg border border-border bg-surface-raised p-5">
				{#if editingRoll}
					<div class="space-y-4">
						<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
							<Input label="Roll ID" bind:value={editRollId} />
							<Input
								label="Frame Count"
								bind:value={editFrameCount}
								type="number"
								placeholder="36"
								hint={editFrameCountHint}
							/>
						</div>
						<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
							<Select
								label="Camera"
								bind:value={editCameraId}
								options={cameraOptions}
								onchange={handleEditCameraChange}
							/>
							{#if editIsFixedLens && editFixedLens}
								<div>
									<span class="mb-1.5 block text-xs font-medium text-text-muted">Default Lens</span>
									<div class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text-muted">
										{lensDisplayName(editFixedLens)} <span class="text-text-faint">(fixed)</span>
									</div>
								</div>
							{:else}
								<Select label="Default Lens" bind:value={editLensId} options={editLensOptions} />
							{/if}
						</div>
						<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
							<Select label="Film Stock" bind:value={editFilmStockId} options={editFilmStockOptions} />
							<Select label="Push/Pull" bind:value={editPushPull} options={pushPullOptions} />
						</div>
						<Textarea label="Notes" bind:value={editNotes} placeholder="Any notes about this roll..." />
						<div class="flex justify-end gap-2 pt-1">
							<Button
								variant="ghost"
								onclick={() => {
									editingRoll = false;
								}}>Cancel</Button
							>
							<Button variant="primary" onclick={saveEditRoll}>Save</Button>
						</div>
					</div>
				{:else}
					<FilmStrip />
					<div class="flex items-start justify-between">
						<div>
							<div class="mb-2 flex items-center gap-3">
								<span class="text-2xl font-mono font-semibold">{roll.roll_id}</span>
								<Badge badge={roll.badge} groupKey={roll.group_key} />
							</div>
							<div class="flex flex-wrap gap-4 text-sm text-text-muted">
								{#if roll.camera_brand}
									<span>{roll.camera_brand} {roll.camera_model}</span>
								{/if}
								{#if roll.film_stock_brand}
									<span>{roll.film_stock_brand} {roll.film_stock_name}</span>
								{/if}
								{#if roll.lens_brand}
									<span>{roll.lens_brand} {roll.lens_name}</span>
								{/if}
								{#if roll.film_stock_iso}
									<span>ISO {roll.film_stock_iso}</span>
								{/if}
								{#if roll.push_pull}
									<span class="rounded bg-accent/15 px-1.5 py-0.5 text-xs text-accent">
										{roll.push_pull.startsWith('+') ? 'Push' : 'Pull'}
										{roll.push_pull}
									</span>
								{/if}
								{#if roll.frame_count}
									<span>{roll.frame_count} frames</span>
								{/if}
							</div>
							<div class="mt-1.5 flex flex-wrap gap-4 text-xs text-text-faint">
								{#if roll.date_loaded}
									<span>Loaded {roll.date_loaded}</span>
								{/if}
								{#if roll.date_finished}
									<span>Finished shooting {roll.date_finished}</span>
								{/if}
							</div>
							{#if roll.notes}
								<p class="mt-2 text-sm text-text-muted whitespace-pre-wrap">{roll.notes}</p>
							{/if}
						</div>
						<div class="flex flex-col items-end gap-3">
							{#if frameProgress}
								<FrameCounter current={frameProgress.current} total={frameProgress.total} size="lg" />
							{/if}
							<Button size="sm" variant="ghost" onclick={startEditRoll}>Edit</Button>
						</div>
					</div>
				{/if}
			</div>
		</FadeIn>

		<!-- Reusable page sections, ordered by the derived phase below. -->

		{#snippet boardSection()}
			<!-- Hidden while the roll Edit form is open (the guard the old Lifecycle-dates
			     section had): a board save reloads the roll under the unsaved edit form,
			     leaving the form holding pre-mutation values. -->
			{#if !editingRoll}
				<div class="mb-6">
					<ActivityBoard
						{activities}
						hasDevRecord={!!labDev || !!selfDev}
						badge={roll?.badge ?? ''}
						expanded={boardExpanded}
						archiveLocation={roll?.archive_location ?? null}
						archiveNaReason={roll?.archive_na_reason ?? null}
						onToggleExpanded={() => (boardExpandedOverride = !boardExpanded)}
						oneditdate={onEditDate}
						oncleardate={onClearDate}
						onchoosepath={openDev}
						onopendev={() => openDev(labDev ? 'lab' : 'self')}
						oneditarchiving={() => (showArchiveDialog = true)}
					/>
					{#if devNotice}<p class="mt-2 text-xs text-text-faint">{devNotice}</p>{/if}
				</div>
			{/if}
		{/snippet}

		{#snippet framesSection()}
			<section class="mb-6">
				{#if phase === 'shooting' && lastShot}
					<!-- "What settings did I just use?" reference card (shooting phase only). -->
					<div class="mb-3 rounded-lg border border-border bg-surface-raised p-4">
						<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-text-faint">Last shot</h3>
						<div class="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-sm">
							<span class="font-mono text-text">#{lastShot.frame}</span>
							{#if lastShot.aperture}<span class="font-mono text-text-muted">{lastShot.aperture}</span>{/if}
							{#if lastShot.shutter}<span class="font-mono text-text-muted">{lastShot.shutter}</span>{/if}
							{#if lastShot.lensName}<span class="text-text-faint">{lastShot.lensName}</span>{/if}
						</div>
					</div>
				{/if}

				<div class="mb-3 flex items-center justify-between">
					<h2 class="flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
						Frames
						{#if frameProgress}
							<span class="font-mono text-[10px] normal-case tracking-normal text-text-faint">
								{frameProgress.current}/{frameProgress.total}
							</span>
						{/if}
						<div class="flex-1 border-b border-border-subtle"></div>
					</h2>
					<div class="flex items-center gap-2">
						<!-- Strip ↔ table toggle. NATIVE radio inputs (visually hidden) inside the
						     styled labels — the same standard ArchiveDialog documents: a shared name
						     gives single-tab-stop + arrow-key roving + "1 of 2" announcements for
						     free, which aria-only role=radio buttons do not. -->
						<div
							class="inline-flex overflow-hidden rounded-lg border border-border"
							role="radiogroup"
							aria-label="Frames view"
						>
							{#each [{ value: 'strip', label: 'Strip' }, { value: 'table', label: 'Table' }] as const as opt (opt.value)}
								<label
									class="cursor-pointer px-2.5 py-1 text-xs font-medium transition-colors not-first:border-l not-first:border-border has-[:focus-visible]:ring-1 has-[:focus-visible]:ring-accent/50 {shotView ===
									opt.value
										? 'bg-accent text-surface'
										: 'text-text-muted hover:bg-surface-overlay hover:text-text'}"
								>
									<input
										type="radio"
										name="frames-view"
										value={opt.value}
										checked={shotView === opt.value}
										onchange={() => (shotViewOverride = opt.value)}
										class="sr-only"
									/>
									{opt.label}
								</label>
							{/each}
						</div>
						<Button
							size="sm"
							variant="secondary"
							aria-expanded={quickAddVisible}
							aria-label={quickAddVisible ? 'Hide the quick entry form' : 'Show the quick entry form'}
							onclick={() => (quickAddOverride = !quickAddVisible)}
						>
							{quickAddVisible ? 'Hide entry' : 'Show entry'}
						</Button>
						<Button size="sm" variant="secondary" href="/quick-entry?roll={roll?.id}">Quick Entry</Button>
					</div>
				</div>

				{#if showRollFullNudge}
					<div
						class="mb-3 flex flex-wrap items-end justify-between gap-3 rounded-lg border border-accent/30 bg-accent/10 px-4 py-3"
					>
						<div class="flex flex-col gap-2">
							<div>
								<p class="text-sm font-medium text-accent">Roll complete</p>
								<p class="text-xs text-accent/70">
									All {frameProgress?.total} frames shot. When did you finish it?
								</p>
							</div>
							<div class="w-44">
								<Input type="date" label="Finished shooting" class="h-[38px]" bind:value={finishDate} />
							</div>
						</div>
						<div class="flex items-center gap-2">
							<Button
								size="sm"
								variant="primary"
								disabled={!finishDate.trim() || !!finishDateError}
								onclick={markAsShot}>Mark as Shot</Button
							>
							<button
								onclick={() => {
									rollFullDismissed = true;
								}}
								class="px-1 text-lg leading-none text-accent/60 transition-colors hover:text-accent"
								aria-label="Dismiss">&times;</button
							>
						</div>
					</div>
				{:else if frameProgress && shots.length > frameProgress.total}
					<div class="mb-3 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-400">
						More shots ({shots.length}) than the roll's frame count ({frameProgress.total}). This may indicate extra
						frames or a counting error.
					</div>
				{/if}

				<!-- Activity journal — rendered beside Quick Entry when the entry form is
				     open, full-width above the strip when it's hidden (kammerz-n7b). -->
				{#snippet activityPane()}
					<div>
						<h3 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
							Activity log
							<div class="flex-1 border-b border-border-subtle"></div>
						</h3>
						<RollActivity {events} onopendev={openDevFromEvent} />
					</div>
				{/snippet}

				<div class="space-y-4">
					{#if quickAddVisible}
						<div class="grid grid-cols-1 gap-4 lg:grid-cols-[22rem_1fr] lg:items-start">
							<QuickAddBar
								frameNumber={nextFrameNumber}
								lensOptions={shotLensOptions}
								lensId={quickDefaultLensId}
								isFixedLens={isFixedLensCamera}
								fixedLensLabel={fixedLens ? lensDisplayName(fixedLens) : ''}
								saving={quickSaving}
								error={quickError}
								onsave={handleQuickAdd}
							/>
							{@render activityPane()}
						</div>
					{:else}
						{@render activityPane()}
					{/if}
					{#if shotView === 'strip'}
						<FrameStrip frames={frameCells} onselect={handleFrameSelect} onaddextra={() => openAddShotDialog()} />
					{:else}
						<ShotsTable
							shots={orderedShots}
							lensNames={shotLensNames}
							onselect={openShotView}
							onaddextra={() => openAddShotDialog()}
						/>
					{/if}
				</div>
			</section>
		{/snippet}

		<!-- DevelopmentSection renders its own Dialogs, which a FadeIn transform would
		     trap (see frontend-patterns.md) — keep it OUTSIDE FadeIn. The dialogs are
		     also opened from the board's Start/Edit controls and journal dev clicks via
		     the autoPrompt bridge. -->
		{#snippet devSection()}
			<div class="mb-6">
				<DevelopmentSection
					rollId={id}
					{labs}
					bind:labDev
					bind:selfDev
					bind:devStages
					bind:autoPrompt={devAutoPrompt}
					defaultDate={shots.length > 0 ? (shots[shots.length - 1].date ?? '') : (roll?.date_loaded ?? '')}
					negativesDeadline={roll?.negatives_deadline ?? null}
					onchange={() => loadRollData()}
					onpromptcancel={() => {
						devNotice = 'No development record was saved.';
					}}
				/>
			</div>
		{/snippet}

		<!-- Auto phase layout (ADR-0013): the derived phase reorders the sections.
		     Shooting/wrap-up → shots front and centre (the shooting phase adds the
		     reference card + quick entry and collapses the board, all keyed off `phase`
		     inside the sections); done → the board leads, fully expanded and still
		     fully editable — only the page order condenses.
		     devSection is hoisted BELOW the branch so crossing the done boundary never
		     remounts DevelopmentSection (which would drop its dialog state and replay
		     its entrance) — only board/frames swap. -->
		{#if phase === 'done'}
			<FadeIn delay={50}>{@render boardSection()}</FadeIn>
			<FadeIn delay={100}>{@render framesSection()}</FadeIn>
		{:else}
			<FadeIn delay={50}>{@render framesSection()}</FadeIn>
			<FadeIn delay={100}>{@render boardSection()}</FadeIn>
		{/if}
		{@render devSection()}
	</div>
{/if}

<svelte:window onkeydown={handleShotDialogKeydown} />

<!-- Add/Edit Shot Dialog -->
{#if showShotDialog}
	<Dialog
		open={true}
		title={isShotView ? `Shot ${editingShotRow?.frame ?? ''}` : editingShotId ? 'Edit Shot' : 'Add Shot'}
		onclose={closeShotDialog}
	>
		<div class="space-y-4">
			{#if editingShotId}
				<div class="flex items-center justify-between">
					<button
						class="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-border text-text-muted transition-colors hover:bg-surface-overlay hover:text-text disabled:cursor-not-allowed disabled:opacity-40"
						disabled={!hasPrevShot || (!isShotView && (shotNavSaving || !!shotDateError))}
						onclick={goPrevShot}
						aria-label="Previous shot"
						title="Previous shot"
					>
						<ChevronLeft size={16} strokeWidth={2} aria-hidden="true" />
					</button>
					{#if currentShotIdx >= 0}
						<span class="text-xs text-text-faint">Shot {currentShotIdx + 1} of {orderedShots.length}</span>
					{/if}
					<button
						class="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-border text-text-muted transition-colors hover:bg-surface-overlay hover:text-text disabled:cursor-not-allowed disabled:opacity-40"
						disabled={!hasNextShot || (!isShotView && (shotNavSaving || !!shotDateError))}
						onclick={goNextShot}
						aria-label="Next shot"
						title="Next shot"
					>
						<ChevronRight size={16} strokeWidth={2} aria-hidden="true" />
					</button>
				</div>
			{/if}
			{#if isShotView && editingShotRow}
				<!-- Read-only view: every field shown, em-dash for the empties (mirrors the
				     strip's — convention). Edit reveals the form below. -->
				<dl class="grid grid-cols-2 gap-x-4 gap-y-3 sm:grid-cols-3">
					{#snippet viewField(label: string, value: string, mono = true)}
						<div>
							<dt class="mb-0.5 text-xs font-medium text-text-faint">{label}</dt>
							<dd class="text-sm {mono ? 'font-mono' : ''} {value ? 'text-text' : 'text-text-faint'}">
								{value || '—'}
							</dd>
						</div>
					{/snippet}
					{@render viewField('Frame #', editingShotRow.frame)}
					{@render viewField('Aperture', editingShotRow.aperture)}
					{@render viewField('Shutter', editingShotRow.shutter)}
					{@render viewField('Date', editingShotRow.date)}
					{@render viewField('Time', editingShotRow.time)}
					{@render viewField('Location', editingShotRow.location, false)}
					{@render viewField('Lens', viewLensLabel, false)}
					<div class="col-span-2 sm:col-span-3">
						<dt class="mb-0.5 text-xs font-medium text-text-faint">Notes</dt>
						<dd
							class="text-sm whitespace-pre-wrap break-words {editingShotRow.notes ? 'text-text' : 'text-text-faint'}"
						>
							{editingShotRow.notes || '—'}
						</dd>
					</div>
				</dl>
				<div class="flex items-center justify-between gap-2 pt-2">
					<Button variant="danger" onclick={startDeleteShot}>Delete</Button>
					<div class="flex gap-2">
						<Button variant="ghost" onclick={closeShotDialog}>Close</Button>
						<!-- Re-open through openShotEdit so the form and dirty baseline re-seed
						     from the LIVE shot — a bare mode flip would edit the values seeded
						     when the view opened, stale if the roll refreshed since. -->
						<Button variant="primary" onclick={() => editingShot && openShotEdit(editingShot)}>Edit</Button>
					</div>
				</div>
			{:else}
				<div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
					<Input label="Frame #" bind:value={shotFrameNumber} placeholder="1" required />
					<ComboInput
						label="Aperture (f/)"
						placeholder="5.6"
						mono
						options={APERTURE_SUGGESTIONS}
						normalize={normalizeAperture}
						warning={shotAperture && !isRecognizedAperture(shotAperture) ? 'Non-standard f/ value' : ''}
						bind:value={shotAperture}
					/>
					<ComboInput
						label="Shutter Speed"
						placeholder="1/250"
						mono
						options={SHUTTER_SUGGESTIONS}
						normalize={normalizeShutter}
						warning={shotShutterSpeed && !isRecognizedShutter(shotShutterSpeed) ? 'Non-standard shutter speed' : ''}
						bind:value={shotShutterSpeed}
					/>
				</div>
				<div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
					<Input type="date" label="Date" class="h-[38px]" bind:value={shotDate} />
					<TimeInput label="Time" bind:value={shotTime} />
					<Input label="Location" bind:value={shotLocation} placeholder="Central Park" />
				</div>

				<!-- Lens Selection -->
				{#if isFixedLensCamera && fixedLens}
					<div>
						<span class="mb-1.5 block text-xs font-medium text-text-muted">Lens</span>
						<div class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text-muted">
							{lensDisplayName(fixedLens)} <span class="text-text-faint">(fixed)</span>
						</div>
					</div>
				{:else}
					<Select label="Lens" bind:value={shotLensId} options={shotLensOptions} />
				{/if}

				<Textarea label="Notes" bind:value={shotNotes} placeholder="Any notes about this shot..." />

				{#if shotError}
					<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{shotError}</div>
				{/if}
				<div class="flex items-center justify-between gap-2 pt-2">
					{#if editingShotId}
						<Button variant="danger" onclick={startDeleteShot}>Delete</Button>
					{:else}
						<span></span>
					{/if}
					<div class="flex gap-2">
						<!-- Edit mode is reached FROM the read-only view, so Cancel returns
						     there (discarding edits by re-seeding from the live shot) instead
						     of closing the whole dialog; add mode just closes. -->
						<Button
							variant="ghost"
							onclick={() => {
								if (editingShot) openShotView(editingShot);
								else closeShotDialog();
							}}>Cancel</Button
						>
						{#if !editingShotId}
							<Button variant="ghost" disabled={!!shotDateError} onclick={handleSaveShotAndNext}>Save & Next</Button>
						{/if}
						<Button variant="primary" disabled={!!shotDateError} onclick={handleSaveShot}>
							{editingShotId ? 'Save' : 'Add Shot'}
						</Button>
					</div>
				</div>
			{/if}
		</div>
	</Dialog>
{/if}

<!-- Delete Roll Confirmation -->
<ConfirmDialog
	bind:open={showDeleteConfirm}
	title="Delete Roll"
	message={roll ? `Permanently delete roll ${roll.roll_id}? This cannot be undone.` : ''}
	confirmLabel="Delete Roll"
	onconfirm={confirmDelete}
	oncancel={() => {}}
/>

<!-- Delete Shot Confirmation -->
{#if deletingShotId !== null}
	<ConfirmDialog
		open={true}
		title="Delete Shot"
		message="Permanently delete this shot? This cannot be undone."
		confirmLabel="Delete Shot"
		onconfirm={confirmDeleteShot}
		oncancel={() => {
			deletingShotId = null;
		}}
	/>
{/if}

<!-- Board: set/change a lifecycle date -->
{#if dateEdit}
	<DateConfirm
		open={true}
		title={dateEdit.title}
		value={dateEdit.current ?? todayLocal()}
		confirmLabel="Save"
		hint="Pick the date this happened — you can change or clear it later from the activity board."
		onconfirm={confirmDateEdit}
		oncancel={() => {
			dateEdit = null;
		}}
	/>
{/if}

<!-- Board: confirm clearing a set lifecycle date (backward move) -->
{#if pendingClear}
	<ConfirmDialog
		open={true}
		title="Clear date"
		message={`Clear the ${pendingClear.label}? This may move the roll back.`}
		confirmLabel="Clear"
		variant="primary"
		onconfirm={confirmClearDate}
		oncancel={() => {
			pendingClear = null;
		}}
	/>
{/if}

<!-- Archiving editor — {#if}-gated like every other dialog on this page, so the
     instance (and its seed effect) exists only while open. -->
{#if showArchiveDialog}
	<ArchiveDialog
		open={true}
		dateArchived={roll?.date_archived ?? null}
		location={roll?.archive_location ?? null}
		na={roll?.archive_na ?? false}
		reason={roll?.archive_na_reason ?? null}
		onsave={handleArchiveSave}
		onclose={() => {
			showArchiveDialog = false;
		}}
	/>
{/if}

<!-- Confirm clearing a set archive date (backward move). The copy must say when
     N/A (+ reason) is what's being applied — 'Remove the archived date?' alone
     would misrepresent an archived→N/A switch as a pure deletion. -->
{#if pendingArchiveClear}
	<ConfirmDialog
		open={true}
		title={pendingArchiveClear.archive_na ? 'Replace archived date with N/A' : 'Remove archived date'}
		message={pendingArchiveClear.archive_na
			? 'Remove the archived date and mark archiving as not applicable? This may move the roll back.'
			: 'Remove the archived date? This may move the roll back.'}
		confirmLabel={pendingArchiveClear.archive_na ? 'Apply' : 'Remove'}
		variant="primary"
		onconfirm={() => {
			const p = pendingArchiveClear!;
			pendingArchiveClear = null;
			void patchRoll(p);
		}}
		oncancel={() => {
			pendingArchiveClear = null;
		}}
	/>
{/if}
