<script lang="ts">
	import { untrack } from 'svelte';
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
	import InlineNotice from '$lib/components/ui/InlineNotice.svelte';
	import RollStatusControl from '$lib/components/rolls/RollStatusControl.svelte';
	import FrameStrip from '$lib/components/rolls/FrameStrip.svelte';
	import QuickAddBar from '$lib/components/rolls/QuickAddBar.svelte';
	import RollActivity from '$lib/components/rolls/RollActivity.svelte';
	import RollTimeline from '$lib/components/rolls/RollTimeline.svelte';
	import FilmStrip from '$lib/components/ui/FilmStrip.svelte';
	import FrameCounter from '$lib/components/ui/FrameCounter.svelte';
	import { getRollDetail, updateRoll, deleteRoll } from '$lib/api/rolls';
	import { updateLabDev, updateSelfDev } from '$lib/api/development';
	import type { DateTarget, TimelineMilestone } from '$lib/utils/timeline';
	import { logShot } from '$lib/utils/shot-entry';
	import { listCameras } from '$lib/api/cameras';
	import { listFilmStocks } from '$lib/api/film-stocks';
	import { listLenses } from '$lib/api/lenses';
	import { createShot, updateShot, deleteShot, suggestNextFrame } from '$lib/api/shots';
	import { listLabs } from '$lib/api/labs';
	import { lensDisplayName, buildLensOptions } from '$lib/utils/lens';
	import { buildFrameCells } from '$lib/utils/frames';
	import { buildShotUpdatePayload, shotFormsEqual, type ShotFormFields } from '$lib/utils/shot-form';
	import { buildCameraLabels } from '$lib/utils/disambiguate';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import {
		statusConfig,
		getDevPath,
		getFlowForPath,
		getPathLabel,
		devKindForStatus,
		getStatusLabel,
		labFlow,
		selfFlow,
		type DevAutoPrompt
	} from '$lib/utils/status';
	import { buildRollTimeline, readDateTarget, STATUS_DATE_TARGET } from '$lib/utils/timeline';
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
		RollStatus,
		PushPull,
		LensMount,
		RollEvent
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

	// Roll edit mode
	let editingRoll = $state(false);
	let editRollId = $state('');
	let editCameraId = $state('');
	let editFilmStockId = $state('');
	let editLensId = $state('');
	let editFrameCount = $state('');
	let editPushPull = $state('');
	let editNotes = $state('');
	let editDateLoaded = $state('');
	let editDateFinished = $state('');
	let editDateScanned = $state('');
	let editDatePostProcessed = $state('');
	let editDateArchived = $state('');

	// Shot state
	let shots: Shot[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let shotLensMap: Record<number, number[]> = $state({});
	let lensMounts: LensMount[] = $state([]);
	let showShotDialog = $state(false);
	let editingShotId: number | null = $state(null);
	let deletingShotId: number | null = $state(null);

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
	const hasEditDateError = $derived(
		!!(
			dateFieldError(editDateLoaded) ||
			dateFieldError(editDateFinished) ||
			dateFieldError(editDateScanned) ||
			dateFieldError(editDatePostProcessed) ||
			dateFieldError(editDateArchived)
		)
	);

	// Development state (shared with DevelopmentSection component)
	let labs: Lab[] = $state([]);
	let labDev: DevelopmentLab | null = $state(null);
	let selfDev: DevelopmentSelf | null = $state(null);
	let devStages: DevStage[] = $state([]);
	let devAutoPrompt: DevAutoPrompt | null = $state(null);
	// Inline notice under the chevron bar — set when a prompt-opened dev dialog is
	// cancelled, so a dropped status click is acknowledged instead of silent.
	let statusNotice = $state('');
	// Transient notice for a status the BACKEND auto-changed as a side effect of a
	// mutation (shot add/delete, dev create/update/delete, Timeline date edit). The
	// backend's auto-sync repaints the chevron/Badge silently otherwise — this makes
	// the invisible change legible. Suppressed for explicit chevron clicks / the
	// roll-full nudge, where the user already drove (and saw) the move (kammerz-9xg).
	let autoStatusNotice = $state('');
	// Bumped on each auto-notice so InlineNotice restarts its dismiss timer even when
	// two consecutive auto-changes produce the same text.
	let autoStatusNoticeSeq = $state(0);

	// Path-aware status flow
	const devPath = $derived(roll ? getDevPath(roll.status as RollStatus, !!labDev, !!selfDev) : ('undecided' as const));
	const statusFlow = $derived(getFlowForPath(devPath));
	const pathLabel = $derived(getPathLabel(devPath));

	// Path-aware lifecycle timeline: each reached milestone's date, editable inline
	// via RollTimeline WITHOUT moving the status (kammerz-2u8). A not-yet-reached
	// milestone is non-editable — its date is recorded by advancing the status, not
	// back-filled here (kammerz-fxl).
	const timeline = $derived(roll ? buildRollTimeline(roll, labDev, selfDev, devPath) : []);

	// Helper: the current value of a status's target date (for the forward+empty check).
	// STATUS_DATE_TARGET and the read dispatch live in timeline.ts so the status→date
	// facts have a single source of truth (see kammerz-mfj).
	function targetDate(t: DateTarget): string | null {
		if (!roll) return null;
		return readDateTarget(t, roll, labDev, selfDev);
	}

	// Status backward-move confirmation
	let pendingStatus: RollStatus | null = $state(null);
	const currentStatusIdx = $derived(roll ? statusFlow.indexOf(roll.status as RollStatus) : -1);

	// Confirm-on-transition prompt state. The label is derived from the pending
	// status so it can never drift out of sync.
	let datePromptOpen = $state(false);
	let datePromptStatus: RollStatus | null = $state(null);
	const datePromptLabel = $derived.by(() => (datePromptStatus ? statusConfig[datePromptStatus].label : ''));

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
	const showRollFullNudge = $derived(
		roll?.status === 'shooting' && frameProgress !== null && shots.length >= frameProgress.total && !rollFullDismissed
	);

	// Status auto-sync (advance on create, revert on delete) is handled by the
	// backend commands transactionally. The frontend just calls loadRollData()
	// after mutations (via DevelopmentSection's onchange) to pick up new status.

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
			return '120 film: 6\u00d74.5=15 \u00b7 6\u00d76=12 \u00b7 6\u00d77=10 \u00b7 6\u00d78=9 \u00b7 6\u00d79=8';
		}
		if (['4x5', '5x7', '8x10'].includes(matchingFormat ?? '') && !editFrameCount) {
			return 'Sheet film: total sheets loaded (e.g. 6 holders \u00d7 2 = 12)';
		}
		return undefined;
	});

	// Shot-level lens dropdown options (uses the saved camera, not the edit form camera)
	const shotLensOptions = $derived(buildLensOptions(allLenses, selectedCamera, 'No lens', lensMounts));

	// Activity journal events (populated from /detail after each load)
	let events: RollEvent[] = $state([]);

	// Frame cells: map shots onto numbered slots, extras appended after (shared util).
	const frameCells = $derived(buildFrameCells(shots, roll?.frame_count ?? null));
	const nextFrameNumber = $derived(frameCells.find((c) => c.isNext)?.frameNumber ?? '');

	// Shots in FrameStrip order (skips empty slots) — for in-dialog prev/next nav.
	const orderedShots = $derived(frameCells.filter((c) => c.shot).map((c) => c.shot!));
	const currentShotIdx = $derived(editingShotId == null ? -1 : orderedShots.findIndex((s) => s.id === editingShotId));
	const hasPrevShot = $derived(currentShotIdx > 0);
	const hasNextShot = $derived(currentShotIdx >= 0 && currentShotIdx < orderedShots.length - 1);

	// QuickAddBar save state
	let quickSaving = $state(false);
	let quickError = $state('');

	// QuickAddBar visibility: hidden by default on archived rolls, shown otherwise.
	// null = follow the default; true/false = user override.
	let quickAddOverride = $state<boolean | null>(null);
	const quickAddVisible = $derived(quickAddOverride ?? roll?.status !== 'archived');

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
	// page can change them. That split is the whole point of this page's load:
	// shot/roll/dev mutations re-pull only the roll's own /detail, not the
	// rarely-changing reference lists.
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

	// Context hint passed into loadRollData so the auto-change notice can describe
	// the side effect WITHOUT guessing its cause — each mutation call site knows what
	// it did. 'explicit' = a user-driven status move (chevron / nudge): suppress the
	// notice even when the status changes. The rest name the mutation that ran so the
	// message can be specific; an undiffed status (no change) shows nothing regardless.
	type ReloadReason = 'navigate' | 'explicit' | 'shot-add' | 'shot-delete' | 'dev' | 'roll-edit' | 'timeline';

	// Direction of an auto-change, judged WITHIN a single dev flow. The canonical full
	// status order interleaves the lab and self branches (at-lab, lab-done, developing,
	// developed), so a cross-flow move (e.g. recording a lab dev on a roll stranded at
	// 'developing' snaps it to 'at-lab') would read as "backward" against that flat order
	// even though it isn't a revert. So we only call it forward/back when BOTH statuses sit
	// on the same flow; a cross-flow or off-flow pair returns null and the message stays
	// neutral ("is now"), which is always true regardless of direction.
	function changeDirection(prev: RollStatus, next: RollStatus): 'forward' | 'back' | null {
		for (const flow of [labFlow, selfFlow]) {
			const p = flow.indexOf(prev);
			const n = flow.indexOf(next);
			if (p !== -1 && n !== -1) return n > p ? 'forward' : 'back';
		}
		return null;
	}

	// Build the auto-change notice text. The verb states direction when it's
	// unambiguous (same flow) and stays neutral otherwise. The optional trailing
	// clause only names a cause the page can GUARANTEE from the mutation that ran and
	// the resulting status — never something it would have to infer. In particular it
	// makes no claim about WHY a dev/timeline reload moved the status, because a single
	// 'dev' reload covers create, edit (a cleared date reverts without removing the
	// record — kammerz-3wg/PR #66) and delete; asserting "record removed" or "date
	// cleared" there would lie for the edit case.
	function statusChangeMessage(prev: RollStatus, next: RollStatus, reason: ReloadReason): string {
		const label = getStatusLabel(next);
		const dir = changeDirection(prev, next);
		const verb = dir === 'forward' ? 'advanced to' : dir === 'back' ? 'moved back to' : 'is now';
		// Guaranteed causes only: a first shot is the sole thing that lands a roll at
		// Shooting via shot-add, and emptying the shot list is the sole thing that
		// reverts to Loaded via shot-delete. Everything else stays cause-free.
		let cause = '.';
		if (reason === 'shot-add' && next === 'shooting') cause = ' — the first shot was logged.';
		else if (reason === 'shot-delete' && next === 'loaded') cause = ' — the roll has no shots left.';
		return `Status ${verb} ${label}${cause}`;
	}

	async function loadRollData(reason: ReloadReason = 'navigate') {
		// Clear any stale error: this component instance is reused across [id]
		// changes (the $effect re-runs loadRollData()), so a prior failure must
		// not leak into the next roll's view. The cancel notice is transient for
		// the same reason — any successful mutation reload supersedes it.
		error = '';
		statusNotice = '';
		// Navigation and explicit user-driven moves clear any stale auto-change notice
		// (a prior roll's, or one superseded by an intentional chevron click); auto
		// reloads keep it until they either set a new one or leave it as-is.
		if (reason === 'navigate' || reason === 'explicit') autoStatusNotice = '';
		// Snapshot the status BEFORE the fetch so an auto-change can be detected by
		// comparing it to the freshly-loaded value below.
		const prevStatus = roll?.status as RollStatus | undefined;
		try {
			// The composite /detail endpoint collapses the six roll-scoped
			// round-trips (roll, shots, shot-lens pairs, lab/self dev, dev stages)
			// into one. Reference catalogs are loaded separately in loadRefData().
			const detail = await getRollDetail(id);
			roll = detail.roll;
			// Reset per-roll nudge state ONCE per roll: reflect an already-set
			// date_finished (e.g. entered via the Edit form) instead of misleadingly
			// showing today, and clear a stale roll-full dismissal — but NOT on
			// same-roll mutation reloads, which would clobber an in-progress finish-date
			// edit or resurrect a banner the user just dismissed (kammerz-hf4).
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

			// Surface a backend auto-sync as the LAST step — after the roll data is
			// applied — so a message-building hiccup (e.g. a status the frontend union
			// doesn't yet know) can never strand the page on stale data. Suppressed for
			// 'explicit'/'navigate' (user-driven moves and page loads, which carry no
			// meaningful "prev" to diff against).
			const newStatus = roll.status as RollStatus;
			if (prevStatus && prevStatus !== newStatus && reason !== 'navigate' && reason !== 'explicit') {
				autoStatusNotice = statusChangeMessage(prevStatus, newStatus, reason);
				autoStatusNoticeSeq += 1;
			}
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
			const lastShot = shots[shots.length - 1];
			if (lastShot.date) shotDate = lastShot.date;
		} else if (roll?.date_loaded) {
			shotDate = roll.date_loaded;
		}

		// Smart lens default: fixed lens > last-used on roll > roll default > camera default
		if (fixedLens) {
			shotLensId = String(fixedLens.id);
		} else if (shots.length > 0) {
			const lastShot = shots[shots.length - 1];
			const lastLensIds = shotLensMap[lastShot.id] ?? [];
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

	function openEditShotDialog(shot: Shot) {
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
		showShotDialog = true;
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
				await loadRollData('shot-add');
			} catch (err) {
				shotError = err instanceof Error ? err.message : String(err);
				return; // stay on this shot — the user can see and fix the failure
			} finally {
				shotNavSaving = false;
			}
		}
		const fresh = orderedShots.find((s) => s.id === target.id);
		if (fresh) openEditShotDialog(fresh);
	}

	function goPrevShot() {
		if (hasPrevShot) void navigateToShot(-1);
	}
	function goNextShot() {
		if (hasNextShot) void navigateToShot(1);
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
			showShotDialog = false;
			resetShotForm();
			await loadRollData('shot-add');
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
			await loadRollData('shot-add');
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
			await loadRollData('shot-delete');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	// Commit a status change. When `date` is provided (from the prompt or the
	// roll-full nudge) and the move is a forward advance, also write it to the
	// status's target record. Backward moves never write a date.
	async function updateStatus(status: RollStatus, date?: string) {
		error = '';
		statusNotice = '';
		try {
			const patch: Partial<RollInsert> = { status };
			const target = STATUS_DATE_TARGET[status];
			const targetIdx = statusFlow.indexOf(status);
			const advancing = currentStatusIdx === -1 || targetIdx > currentStatusIdx;

			// Roll-owned dates go in the same PATCH as the status. Use Object.assign
			// with a computed key (not `patch[target.field] = date`) — a union-keyed
			// index assignment trips TS2322 ("not assignable to never").
			if (advancing && date && target?.kind === 'roll') {
				Object.assign(patch, { [target.field]: date });
			}
			await updateRoll(id, patch);

			// Dev-owned dates (lab/self) are a follow-up write to the dev record —
			// non-atomic with the status PATCH above. The status is already committed,
			// so surface a failed date write via `error` but STILL refresh below —
			// otherwise the UI would keep showing the old status. The date can then be
			// re-entered from the Timeline. (Roll-owned dates were co-batched above.)
			try {
				if (advancing && date && target && target.kind !== 'roll') {
					await writeDateTarget(target, date);
				}
			} catch (err) {
				error = err instanceof Error ? err.message : String(err);
			}

			await loadRollData('explicit');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	// Write a milestone date (or null to clear) to whichever record owns it. Shared
	// by the inline Timeline editor and the confirm-on-transition flow so the
	// roll/lab/self dispatch lives in one place. A lab/self target whose dev record
	// doesn't exist is a no-op (the date lives on that record).
	async function writeDateTarget(t: DateTarget, date: string | null): Promise<void> {
		if (t.kind === 'roll') {
			await updateRoll(id, { [t.field]: date });
		} else if (t.kind === 'lab' && labDev) {
			await updateLabDev(labDev.id, { [t.field]: date });
		} else if (t.kind === 'self' && selfDev) {
			await updateSelfDev(selfDev.id, { [t.field]: date });
		}
	}

	// Persist an inline Timeline milestone-date edit, then refresh. Distinct from a
	// chevron status move: this only writes the date to its owning record and never
	// changes the status (a roll-owned date is inert; clearing a dev-owned date can
	// trigger the backend's data-driven revert, which the 'timeline' reload surfaces
	// via autoStatusNotice — kammerz-2u8/3wg).
	async function saveTimelineDate(milestone: TimelineMilestone, date: string | null) {
		error = '';
		try {
			await writeDateTarget(milestone.target, date);
			await loadRollData('timeline');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	// Hover/aria hint for each status chevron, derived from the SAME branches
	// handleStatusClick takes — so the hint can never describe behavior the click
	// won't perform. Keep these two in lockstep when changing click logic.
	function statusHint(status: RollStatus): string {
		const label = statusConfig[status].label;
		const targetIdx = statusFlow.indexOf(status);
		const devKind = devKindForStatus(status);
		const devRecordMissing = (devKind === 'lab' && !labDev) || (devKind === 'self' && !selfDev);
		// Backward move stays first, mirroring handleStatusClick: an earlier lab/self
		// chevron with no record still opens the Move-Back confirm (the devKind guard
		// there only runs after the backward early-return), so it must NOT be described
		// as recording development.
		if (currentStatusIdx !== -1 && targetIdx < currentStatusIdx) {
			return `Move back to ${label} (asks to confirm)`;
		}
		// A current/forward lab/self status with no backing dev record → clicking opens
		// the dev form (handleStatusClick's devKind guard). This includes the roll
		// stranded AT a lab/self status with no record — the one orphan-recovery state
		// the bare "Current status" label left unexplained (kammerz-6ih).
		if (devRecordMissing) {
			const kindLabel = devKind === 'lab' ? 'lab' : 'self';
			return roll?.status === status
				? `Record ${kindLabel} development for ${label} (current status)`
				: `Record ${kindLabel} development to move to ${label}`;
		}
		if (roll?.status === status) return `Current status: ${label}`;
		// Forward into a date-bearing status whose date isn't recorded yet → prompts.
		const target = STATUS_DATE_TARGET[status];
		if (target && !targetDate(target)) return `Move to ${label} (asks for a date)`;
		return `Move to ${label}`;
	}

	function handleStatusClick(status: RollStatus) {
		if (!roll) return;
		statusNotice = '';
		const targetIdx = statusFlow.indexOf(status);
		// Backward move — confirm, never touch dates.
		if (currentStatusIdx !== -1 && targetIdx < currentStatusIdx) {
			pendingStatus = status;
			return;
		}
		// Forward into a lab/self status whose dev record doesn't exist yet → open the matching
		// dev dialog instead of advancing. Creating the record auto-syncs the status (backend),
		// so a status is never stranded at at-lab/lab-done/developing/developed with no backing
		// record (and no way to capture its dates). Mirrors the "Develop" menu (chooseDevPath).
		// The clicked status rides along as `target` so the dialog can seed the date
		// field that lands the roll there (e.g. a "Lab Done" click pre-fills Date
		// Received — otherwise saving would stop one rung short at At Lab).
		const devKind = devKindForStatus(status);
		if (devKind === 'lab' && !labDev) {
			devAutoPrompt = { kind: 'lab', target: status };
			return;
		}
		if (devKind === 'self' && !selfDev) {
			devAutoPrompt = { kind: 'self', target: status };
			return;
		}
		// Forward into a date-bearing status whose date isn't recorded yet → prompt. The dev
		// record (for lab/self targets) is guaranteed to exist here, gated by the guard above.
		const target = STATUS_DATE_TARGET[status];
		if (target && !targetDate(target)) {
			datePromptStatus = status;
			datePromptOpen = true;
			return;
		}
		// Otherwise advance directly: no date target, or the date is already recorded.
		updateStatus(status);
	}

	function confirmDatePrompt(date: string | null) {
		const status = datePromptStatus;
		datePromptOpen = false;
		datePromptStatus = null;
		if (status) updateStatus(status, date ?? undefined);
	}

	function cancelDatePrompt() {
		datePromptOpen = false;
		datePromptStatus = null;
	}

	// Commit to a development path from the "Develop" chevron. Reuses the dev-dialog
	// auto-prompt wiring: opening + saving a dev record makes getDevPath resolve to
	// this path, the backend auto-syncs status, and the chevron bar re-renders.
	function chooseDevPath(path: 'lab' | 'self') {
		statusNotice = '';
		devAutoPrompt = { kind: path };
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
		editDateLoaded = roll.date_loaded ?? '';
		editDateFinished = roll.date_finished ?? '';
		editDateScanned = roll.date_scanned ?? '';
		editDatePostProcessed = roll.date_post_processed ?? '';
		editDateArchived = roll.date_archived ?? '';
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
		if (hasEditDateError) {
			error = 'Fix the highlighted dates.';
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
				notes: editNotes || null,
				date_loaded: editDateLoaded.trim() || null,
				date_finished: editDateFinished.trim() || null,
				date_scanned: editDateScanned.trim() || null,
				date_post_processed: editDatePostProcessed.trim() || null,
				date_archived: editDateArchived.trim() || null
			});
			editingRoll = false;
			await loadRollData('roll-edit');
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
		devAutoPrompt = { kind: refKind === 'lab_dev' ? 'lab' : 'self' };
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
			await loadRollData('shot-add');
			return true;
		} catch (err) {
			quickError = err instanceof Error ? err.message : String(err);
			return false;
		} finally {
			quickSaving = false;
		}
	}

	// FrameStrip selection: filled frame → edit dialog; open slot → add dialog pre-seeded
	function handleFrameSelect(frameNumber: string, shot: Shot | null) {
		if (shot) {
			openEditShotDialog(shot);
		} else {
			openAddShotDialog(frameNumber);
		}
	}

	// Page load: on mount and whenever the roll id changes (navigation), fetch
	// reference catalogs and roll detail together, gating `loading` until BOTH
	// resolve. This keeps reference data fresh on every page entry while never
	// rendering the roll with empty dropdowns / lens lookups. Mutations bypass this
	// and call loadRollData() directly, so they refresh only the roll's /detail.
	//
	// Track ONLY `id` (the `void id` read), then untrack the loaders: Svelte 5
	// tracks every reactive read in an effect's synchronous run — INCLUDING the
	// sync prefix of an async fn it calls. loadRollData()'s prefix reads `roll`
	// (the prevStatus snapshot) and then rewrites `roll` post-fetch, so without
	// untrack the effect depends on a value it mutates → unbounded re-fetch loop
	// on every roll-detail visit (kammerz-8k5).
	$effect(() => {
		void id;
		untrack(() => {
			// Re-apply the per-roll QuickAddBar default (hidden iff archived) on every
			// roll change, so a manual show/hide on one roll doesn't leak to the next.
			quickAddOverride = null;
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
						<div class="space-y-2">
							<div class="flex items-center gap-3">
								<span class="text-xs font-semibold uppercase tracking-wider text-text-faint">Lifecycle dates</span>
								<div class="flex-1 border-b border-border-subtle"></div>
							</div>
							<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
								<Input type="date" label="Loaded" class="h-[38px]" bind:value={editDateLoaded} />
								<Input type="date" label="Finished shooting" class="h-[38px]" bind:value={editDateFinished} />
								<Input type="date" label="Scanned" class="h-[38px]" bind:value={editDateScanned} />
								<Input type="date" label="Post-processed" class="h-[38px]" bind:value={editDatePostProcessed} />
								<Input type="date" label="Archived" class="h-[38px]" bind:value={editDateArchived} />
							</div>
						</div>
						<Textarea label="Notes" bind:value={editNotes} placeholder="Any notes about this roll..." />
						<div class="flex justify-end gap-2 pt-1">
							<Button
								variant="ghost"
								onclick={() => {
									editingRoll = false;
								}}>Cancel</Button
							>
							<Button variant="primary" disabled={hasEditDateError} onclick={saveEditRoll}>Save</Button>
						</div>
					</div>
				{:else}
					<FilmStrip />
					<div class="flex items-start justify-between">
						<div>
							<div class="mb-2 flex items-center gap-3">
								<span class="text-2xl font-mono font-semibold">{roll.roll_id}</span>
								<Badge status={roll.status} />
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

		<!-- Status Control -->
		<FadeIn delay={50}>
			<div class="mb-6">
				<RollStatusControl
					{statusFlow}
					currentStatus={roll.status}
					{currentStatusIdx}
					{devPath}
					{pathLabel}
					hintFor={statusHint}
					onmove={handleStatusClick}
					onchoosepath={chooseDevPath}
				/>
				{#if statusNotice}<p class="mt-2 text-xs text-text-faint">{statusNotice}</p>{/if}
				{#if autoStatusNotice}
					<div class="mt-2">
						<InlineNotice bind:message={autoStatusNotice} seq={autoStatusNoticeSeq} />
					</div>
				{/if}
			</div>
		</FadeIn>

		<!-- Lifecycle dates — inline milestone-date editing, separate from the status
		     chevron bar above: the chevrons CHANGE status; this CORRECTS the date each
		     reached step happened, without moving status (kammerz-2u8).
		     Hidden while the roll Edit form is open — that form has its own "Lifecycle
		     dates" inputs for the roll-owned dates, so showing both would duplicate the
		     heading and controls on screen.
		     Deliberately NOT wrapped in FadeIn — RollTimeline renders its own DateConfirm
		     dialog, and a FadeIn transform creates a containing block that traps a
		     fixed-positioned dialog. This matches the DevelopmentSection precedent below;
		     both Dialog-rendering components stay outside FadeIn (see frontend-patterns.md). -->
		{#if !editingRoll}
			<div class="mb-6">
				<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
					Lifecycle dates
					<div class="flex-1 border-b border-border-subtle"></div>
				</h2>
				<RollTimeline milestones={timeline} onedit={saveTimelineDate} />
			</div>
		{/if}

		<!-- Frames — front-and-center: most info + the Quick Entry control. Placed
		     above Development and Activity (kammerz-60f). -->
		<FadeIn delay={100}>
			<section class="mb-6">
				<div class="mb-3 flex items-center justify-between">
					<h2 class="flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
						Frames
						{#if frameProgress}
							<span class="font-mono text-[10px] normal-case tracking-normal text-text-faint">
								{frameProgress.current}/{frameProgress.total}
							</span>
							<div class="flex-1 border-b border-border-subtle"></div>
						{:else}
							<div class="flex-1 border-b border-border-subtle"></div>
						{/if}
					</h2>
					<div class="flex items-center gap-2">
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
								onclick={() => updateStatus('shot', finishDate)}>Mark as Shot</Button
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
							Activity
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
					<FrameStrip frames={frameCells} onselect={handleFrameSelect} onaddextra={() => openAddShotDialog()} />
				</div>
			</section>
		</FadeIn>

		<!-- DevelopmentSection: the compact Development panel + its create/edit/delete dialogs.
		     Dialogs are also opened from chevron clicks and from journal dev-event clicks via the autoPrompt bridge.
		     NOT wrapped in FadeIn — it renders its own Dialogs, which a FadeIn transform would trap (see frontend-patterns.md). -->
		<div class="mb-6">
			<DevelopmentSection
				rollId={id}
				{labs}
				bind:labDev
				bind:selfDev
				bind:devStages
				bind:autoPrompt={devAutoPrompt}
				currentStatus={roll?.status ?? null}
				defaultDate={shots.length > 0 ? (shots[shots.length - 1].date ?? '') : (roll?.date_loaded ?? '')}
				negativesDeadline={roll?.negatives_deadline ?? null}
				onchange={() => loadRollData('dev')}
				onpromptcancel={() => {
					statusNotice = 'Status unchanged — no development record was saved.';
				}}
			/>
		</div>
	</div>
{/if}

<svelte:window onkeydown={handleShotDialogKeydown} />

<!-- Add/Edit Shot Dialog -->
{#if showShotDialog}
	<Dialog
		open={true}
		title={editingShotId ? 'Edit Shot' : 'Add Shot'}
		onclose={() => {
			showShotDialog = false;
			resetShotForm();
		}}
	>
		<div class="space-y-4">
			{#if editingShotId}
				<div class="flex items-center justify-between">
					<button
						class="inline-flex h-8 w-8 items-center justify-center rounded-lg border border-border text-text-muted transition-colors hover:bg-surface-overlay hover:text-text disabled:cursor-not-allowed disabled:opacity-40"
						disabled={!hasPrevShot || shotNavSaving || !!shotDateError}
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
						disabled={!hasNextShot || shotNavSaving || !!shotDateError}
						onclick={goNextShot}
						aria-label="Next shot"
						title="Next shot"
					>
						<ChevronRight size={16} strokeWidth={2} aria-hidden="true" />
					</button>
				</div>
			{/if}
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
					<Button
						variant="danger"
						onclick={() => {
							const sid = editingShotId;
							showShotDialog = false;
							resetShotForm();
							deletingShotId = sid;
						}}>Delete</Button
					>
				{:else}
					<span></span>
				{/if}
				<div class="flex gap-2">
					<Button
						variant="ghost"
						onclick={() => {
							showShotDialog = false;
							resetShotForm();
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

<!-- Backward Status Move Confirmation -->
{#if pendingStatus}
	<ConfirmDialog
		open={true}
		title="Move Status Back"
		message={`Move status back to ${statusConfig[pendingStatus].label}? This will revert progress.`}
		confirmLabel="Move Back"
		variant="primary"
		onconfirm={() => {
			updateStatus(pendingStatus!);
			pendingStatus = null;
		}}
		oncancel={() => {
			pendingStatus = null;
		}}
	/>
{/if}

<!-- Forward Status Date Prompt -->
<DateConfirm
	bind:open={datePromptOpen}
	title={`Date for "${datePromptLabel}"`}
	value={todayLocal()}
	confirmLabel="Confirm"
	onconfirm={confirmDatePrompt}
	oncancel={cancelDatePrompt}
/>
