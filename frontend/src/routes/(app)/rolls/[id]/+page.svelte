<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import DateInput from '$lib/components/ui/DateInput.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import DevelopmentSection from '$lib/components/rolls/DevelopmentSection.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import { getRollDetail, updateRoll, deleteRoll } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import { listFilmStocks } from '$lib/api/film-stocks';
	import { listLenses } from '$lib/api/lenses';
	import { createShot, updateShot, deleteShot, suggestNextFrame } from '$lib/api/shots';
	import { listLabs } from '$lib/api/labs';
	import { lensDisplayName, buildLensOptions } from '$lib/utils/lens';
	import { buildCameraLabels } from '$lib/utils/disambiguate';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import { statusConfig, getDevPath, getFlowForPath, getPathLabel, allStatusOrder } from '$lib/utils/status';
	import type { RollWithDetails, RollInsert, Camera, FilmStock, Lens, Shot, Lab, DevelopmentLab, DevelopmentSelf, DevStage, RollStatus, PushPull, LensMount } from '$lib/types';

	const id = $derived(Number(page.params.id));

	// Back navigation: read ?from= param to determine where we came from
	const backRoutes: Record<string, { href: string; label: string }> = {
		developments: { href: '/developments', label: 'Developing' },
		search: { href: '/search', label: 'Search' },
		dashboard: { href: '/', label: 'Dashboard' }
	};
	const fromParam = $derived(page.url.searchParams.get('from'));
	const backNav = $derived(fromParam && backRoutes[fromParam] ? backRoutes[fromParam] : { href: '/rolls', label: 'Rolls' });

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
	let editDateLoaded = $state('');
	let editDateFinished = $state('');
	let editDateFuzzy = $state('');
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

	// Shot form fields
	let shotFrameNumber = $state('');
	let shotAperture = $state('');
	let shotShutterSpeed = $state('');
	let shotDate = $state('');
	let shotLocation = $state('');
	let shotNotes = $state('');
	let shotLensId = $state('');
	let shotError = $state('');

	// Development state (shared with DevelopmentSection component)
	let labs: Lab[] = $state([]);
	let labDev: DevelopmentLab | null = $state(null);
	let selfDev: DevelopmentSelf | null = $state(null);
	let devStages: DevStage[] = $state([]);
	let devAutoPrompt: 'lab' | 'self' | null = $state(null);

	// Path-aware status flow
	const devPath = $derived(
		roll ? getDevPath(roll.status as RollStatus, !!labDev, !!selfDev) : 'undecided' as const
	);
	const statusFlow = $derived(getFlowForPath(devPath));
	const pathLabel = $derived(getPathLabel(devPath));

	// Status backward-move confirmation
	let pendingStatus: RollStatus | null = $state(null);
	const currentStatusIdx = $derived(roll ? statusFlow.indexOf(roll.status as RollStatus) : -1);

	// Frame progress
	const frameProgress = $derived.by(() => {
		if (!roll?.frame_count) return null;
		return { current: shots.length, total: roll.frame_count };
	});

	// Local (not UTC) today as YYYY-MM-DD — used to stamp the finish date.
	function todayLocal(): string {
		const d = new Date();
		const mm = String(d.getMonth() + 1).padStart(2, '0');
		const dd = String(d.getDate()).padStart(2, '0');
		return `${d.getFullYear()}-${mm}-${dd}`;
	}

	// Roll-full nudge state
	let rollFullDismissed = $state(false);
	// Finish date captured at the shooting→shot transition (prefilled with today,
	// editable in the nudge). The shot moment is significant enough to record.
	let finishDate = $state(todayLocal());
	const showRollFullNudge = $derived(
		roll?.status === 'shooting' &&
		frameProgress !== null &&
		shots.length >= frameProgress.total &&
		!rollFullDismissed
	);

	// Development-path picker popover (the live "Develop" chevron in the undecided flow).
	let showDevPathMenu = $state(false);

	// Status auto-sync (advance on create, revert on delete) is handled by the
	// backend commands transactionally. The frontend just calls loadRollData()
	// after mutations (via DevelopmentSection's onchange) to pick up new status.

	const cameraLabels = $derived(buildCameraLabels(cameras));
	const cameraOptions = $derived([
		{ value: '', label: 'Not assigned' },
		...cameras.map((c) => ({ value: String(c.id), label: cameraLabels.get(c.id) ?? `${c.brand} ${c.model}` }))
	]);

	const selectedCamera = $derived(
		roll?.camera_id ? cameras.find((c) => c.id === roll?.camera_id) ?? null : null
	);

	// Fixed-lens camera detection (based on saved roll camera)
	const isFixedLensCamera = $derived(
		selectedCamera ? lensMounts.some((m) => m.id === selectedCamera.lens_mount_id && m.name === 'Fixed Lens') : false
	);
	const fixedLens = $derived(
		isFixedLensCamera && selectedCamera?.default_lens_id
			? allLenses.find((l) => l.id === selectedCamera.default_lens_id) ?? null
			: null
	);

	// Edit-mode: camera selected in the edit form (for reactive film stock / lens filtering)
	const editSelectedCamera = $derived(
		editCameraId ? cameras.find((c) => c.id === Number(editCameraId)) ?? null : null
	);

	// Edit-mode: fixed-lens detection for the camera selected in the edit form
	const editIsFixedLens = $derived(
		editSelectedCamera ? lensMounts.some((m) => m.id === editSelectedCamera.lens_mount_id && m.name === 'Fixed Lens') : false
	);
	const editFixedLens = $derived(
		editIsFixedLens && editSelectedCamera?.default_lens_id
			? allLenses.find((l) => l.id === editSelectedCamera.default_lens_id) ?? null
			: null
	);

	// Camera format → film stock format mapping
	const cameraFormatToStockFormat: Record<string, string> = {
		'35mm': '135',
		'medium format': '120',
		'6x4.5': '120', '6x6': '120', '6x7': '120', '6x8': '120', '6x9': '120',
		'large format': '4x5', '4x5': '4x5', '5x7': '5x7', '8x10': '8x10',
		'instant': 'instant'
	};

	function stockLabel(s: FilmStock): string {
		return `${s.brand} ${s.name} (${s.format}${s.iso ? ', ISO ' + s.iso : ''})`;
	}

	const editFilmStockOptions = $derived.by(() => {
		const matchingFormat = editSelectedCamera
			? cameraFormatToStockFormat[editSelectedCamera.format]
			: null;
		if (!matchingFormat) {
			return [
				{ value: '', label: 'Not assigned' },
				...filmStocks.map((s) => ({ value: String(s.id), label: stockLabel(s) }))
			];
		}
		const matching = filmStocks.filter((s) => s.format === matchingFormat);
		const rest = filmStocks.filter((s) => s.format !== matchingFormat);
		const options: { value: string; label: string; disabled?: boolean }[] = [
			{ value: '', label: 'Not assigned' }
		];
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
		const matchingFormat = editSelectedCamera
			? cameraFormatToStockFormat[editSelectedCamera.format]
			: null;
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

	async function loadRollData() {
		// Clear any stale error: this component instance is reused across [id]
		// changes (the $effect re-runs loadRollData()), so a prior failure must
		// not leak into the next roll's view.
		error = '';
		try {
			// The composite /detail endpoint collapses the six roll-scoped
			// round-trips (roll, shots, shot-lens pairs, lab/self dev, dev stages)
			// into one. Reference catalogs are loaded separately in loadRefData().
			const detail = await getRollDetail(id);
			roll = detail.roll;
			rollFullDismissed = false;
			shots = detail.shots;
			labDev = detail.lab_dev;
			selfDev = detail.self_dev;
			devStages = detail.dev_stages;

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
		shotLocation = '';
		shotNotes = '';
		shotLensId = '';
		shotError = '';
	}

	async function openAddShotDialog() {
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

		try {
			shotFrameNumber = await suggestNextFrame(id);
		} catch {
			shotFrameNumber = '';
		}
		showShotDialog = true;
	}

	function openEditShotDialog(shot: Shot) {
		editingShotId = shot.id;
		shotFrameNumber = shot.frame_number;
		shotAperture = shot.aperture ?? '';
		shotShutterSpeed = shot.shutter_speed ?? '';
		shotDate = shot.date ?? '';
		shotLocation = shot.location ?? '';
		shotNotes = shot.notes ?? '';
		const ids = shotLensMap[shot.id] ?? [];
		shotLensId = ids.length > 0 ? String(ids[0]) : '';
		shotError = '';
		showShotDialog = true;
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
				await updateShot(editingShotId, {
					frame_number: shotFrameNumber.trim(),
					aperture: shotAperture || null,
					shutter_speed: shotShutterSpeed || null,
					date: shotDate || null,
					location: shotLocation || null,
					notes: shotNotes || null,
					lens_ids: lensIds
				});
			} else {
				await createShot({
					roll_id: id,
					frame_number: shotFrameNumber.trim(),
					aperture: shotAperture || null,
					shutter_speed: shotShutterSpeed || null,
					date: shotDate || null,
					date_fuzzy: null,
					location: shotLocation || null,
					gps_lat: null,
					gps_lon: null,
					notes: shotNotes || null,
					lens_ids: lensIds
				});
			}
			showShotDialog = false;
			resetShotForm();
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
				aperture: shotAperture || null,
				shutter_speed: shotShutterSpeed || null,
				date: shotDate || null,
				date_fuzzy: null,
				location: shotLocation || null,
				gps_lat: null,
				gps_lon: null,
				notes: shotNotes || null,
				lens_ids: lensIds
			});
			await loadRollData();
			// Reset per-shot fields but keep session defaults (date, location, lens)
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
		error = '';
		try {
			await deleteShot(deletingShotId);
			deletingShotId = null;
			await loadRollData();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	function getShotLensDisplay(shotId: number): { name: string; isDefault: boolean } | null {
		const ids = shotLensMap[shotId] ?? [];
		if (ids.length > 0) {
			// Per-shot override
			const names = ids
				.map((lid) => allLenses.find((l) => l.id === lid))
				.filter(Boolean)
				.map((l) => lensDisplayName(l!))
				.join(', ');
			return { name: names, isDefault: false };
		}
		// Fall back to roll default
		if (roll?.lens_id) {
			const defaultLens = allLenses.find((l) => l.id === roll?.lens_id);
			if (defaultLens) {
				return { name: lensDisplayName(defaultLens), isDefault: true };
			}
		}
		return null;
	}

	async function updateStatus(status: RollStatus, finishDate?: string) {
		error = '';
		try {
			const patch: Partial<RollInsert> = { status };
			// Capture the shooting→shot transition date — previously this moment
			// was dropped on the floor. Use the explicit date from the nudge when
			// provided, otherwise stamp today; never overwrite an existing value.
			if (status === 'shot' && roll && !roll.date_finished) {
				patch.date_finished = finishDate || todayLocal();
			}
			await updateRoll(id, patch);
			await loadRollData();
			// Auto-prompt development dialogs
			if (status === 'at-lab' && !labDev && !selfDev) {
				devAutoPrompt = 'lab';
			} else if (status === 'developing' && !selfDev && !labDev) {
				devAutoPrompt = 'self';
			}
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	function handleStatusClick(status: RollStatus) {
		if (!roll) return;
		const targetIdx = statusFlow.indexOf(status);
		if (currentStatusIdx !== -1 && targetIdx < currentStatusIdx) {
			// Backward move — ask for confirmation
			pendingStatus = status;
		} else {
			updateStatus(status);
		}
	}

	// Commit to a development path from the "Develop" chevron. Reuses the dev-dialog
	// auto-prompt wiring: opening + saving a dev record makes getDevPath resolve to
	// this path, the backend auto-syncs status, and the chevron bar re-renders.
	function chooseDevPath(path: 'lab' | 'self') {
		showDevPathMenu = false;
		devAutoPrompt = path;
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
		editDateLoaded = roll.date_loaded ?? '';
		editDateFinished = roll.date_finished ?? '';
		editDateFuzzy = roll.date_fuzzy ?? '';
		editPushPull = roll.push_pull ?? '';
		editNotes = roll.notes ?? '';
		editingRoll = true;
	}

	async function saveEditRoll() {
		error = '';
		try {
			await updateRoll(id, {
				roll_id: editRollId,
				camera_id: editCameraId ? Number(editCameraId) : null,
				film_stock_id: editFilmStockId ? Number(editFilmStockId) : null,
				lens_id: editLensId ? Number(editLensId) : null,
				frame_count: editFrameCount ? parseInt(editFrameCount) : null,
				date_loaded: editDateLoaded || null,
				date_finished: editDateFinished || null,
				date_fuzzy: editDateFuzzy || null,
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
		error = '';
		try {
			await deleteRoll(id);
			goto('/rolls');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	// Page load: on mount and whenever the roll id changes (navigation), fetch
	// reference catalogs and roll detail together, gating `loading` until BOTH
	// resolve. This keeps reference data fresh on every page entry while never
	// rendering the roll with empty dropdowns / lens lookups. The $effect tracks
	// `id` via loadRollData's synchronous read of it. Mutations bypass this and
	// call loadRollData() directly, so they refresh only the roll's /detail.
	$effect(() => {
		loading = true;
		Promise.all([loadRefData(), loadRollData()]).finally(() => {
			loading = false;
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
		<Button variant="danger" onclick={handleDelete}>Delete</Button>
	</PageHeader>

	<div class="p-6">
		{#if error}
			<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}

		<!-- Roll Header -->
		<FadeIn delay={0}>
		<div class="mb-6 rounded-lg border border-border bg-surface-raised p-5">
			{#if editingRoll}
				<div class="space-y-4">
					<div class="grid grid-cols-2 gap-4">
						<Input label="Roll ID" bind:value={editRollId} />
						<Input label="Frame Count" bind:value={editFrameCount} type="number" placeholder="36" hint={editFrameCountHint} />
					</div>
					<Select label="Camera" bind:value={editCameraId} options={cameraOptions} />
					<Select label="Film Stock" bind:value={editFilmStockId} options={editFilmStockOptions} />
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
					<div class="grid grid-cols-2 gap-4">
						<DateInput label="Date Loaded" bind:value={editDateLoaded} />
						<DateInput label="Date Finished" bind:value={editDateFinished} />
					</div>
					<div class="grid grid-cols-2 gap-4">
						<Select label="Push/Pull" bind:value={editPushPull} options={pushPullOptions} />
						<Input label="Fuzzy Date" bind:value={editDateFuzzy} placeholder="e.g. 'early October 2025'" />
					</div>
					<Textarea label="Notes" bind:value={editNotes} placeholder="Any notes about this roll..." />
					<div class="flex justify-end gap-2 pt-1">
						<Button variant="ghost" onclick={() => { editingRoll = false; }}>Cancel</Button>
						<Button variant="primary" onclick={saveEditRoll}>Save</Button>
					</div>
				</div>
			{:else}
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
									{roll.push_pull.startsWith('+') ? 'Push' : 'Pull'} {roll.push_pull}
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
								<span>Finished {roll.date_finished}</span>
							{/if}
						</div>
						{#if roll.date_fuzzy}
							<p class="mt-1 text-xs italic text-text-faint">{roll.date_fuzzy}</p>
						{/if}
						{#if roll.notes}
							<p class="mt-2 text-sm text-text-muted whitespace-pre-wrap">{roll.notes}</p>
						{/if}
					</div>
					<Button size="sm" variant="ghost" onclick={startEditRoll}>Edit</Button>
				</div>
			{/if}
		</div>
		</FadeIn>

		<!-- Status Progression -->
		<FadeIn delay={50}>
		<div class="mb-6">
			<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
				Status
				<div class="flex-1 border-b border-border-subtle"></div>
			</h2>
			{#if pathLabel}
				<p class="mb-1.5 text-[10px] font-medium uppercase tracking-widest text-text-faint/70">{pathLabel}</p>
			{/if}
			<div class="flex items-center gap-[2px]">
				{#each statusFlow as status, idx}
					{@const isFirst = idx === 0}
					{@const isLast = idx === statusFlow.length - 1}
					{@const clipPath = isFirst
						? 'polygon(0 0, calc(100% - 8px) 0, 100% 50%, calc(100% - 8px) 100%, 0 100%)'
						: isLast
							? 'polygon(0 0, 100% 0, 100% 100%, 0 100%, 8px 50%)'
							: 'polygon(0 0, calc(100% - 8px) 0, 100% 50%, calc(100% - 8px) 100%, 0 100%, 8px 50%)'}
					<button
						onclick={() => handleStatusClick(status)}
						style="clip-path: {clipPath}"
						class="py-1.5 text-xs font-medium transition-colors
							{isFirst ? 'pl-3 pr-4' : isLast ? 'pl-4 pr-3' : 'px-4'}
							{roll.status === status
							? 'bg-accent text-surface'
							: idx < currentStatusIdx
								? 'bg-accent/10 text-accent/70 hover:bg-accent/20'
								: 'bg-surface-overlay text-text-muted hover:text-text'}"
					>
						{statusConfig[status].label}
					</button>
					{#if devPath === 'undecided' && status === 'shot'}
						<!-- Live next-step: the development decision happens here, in the flow,
						     rather than as disconnected dead placeholders. Choosing a path
						     opens the matching dev dialog and re-renders the bar to that flow. -->
						<div class="relative">
							<button
								onclick={() => (showDevPathMenu = !showDevPathMenu)}
								aria-haspopup="menu"
								aria-expanded={showDevPathMenu}
								style="clip-path: polygon(0 0, calc(100% - 8px) 0, 100% 50%, calc(100% - 8px) 100%, 0 100%, 8px 50%)"
								class="px-4 py-1.5 text-xs font-medium transition-colors hover:bg-accent/25
									{showDevPathMenu ? 'bg-accent/25 text-accent' : 'bg-accent/15 text-accent'}"
							>
								Develop&nbsp;⌄
							</button>
							{#if showDevPathMenu}
								<!-- click-away catcher -->
								<button
									class="fixed inset-0 z-10 cursor-default"
									aria-label="Close development menu"
									onclick={() => (showDevPathMenu = false)}
								></button>
								<div
									role="menu"
									class="absolute left-1/2 top-full z-20 mt-1.5 -translate-x-1/2 overflow-hidden rounded-lg border border-border bg-surface-overlay shadow-lg"
								>
									<button
										role="menuitem"
										onclick={() => chooseDevPath('lab')}
										class="block w-full whitespace-nowrap px-4 py-2 text-left text-xs font-medium text-text-muted transition-colors hover:bg-accent/15 hover:text-accent"
									>Lab</button>
									<button
										role="menuitem"
										onclick={() => chooseDevPath('self')}
										class="block w-full whitespace-nowrap border-t border-border-subtle px-4 py-2 text-left text-xs font-medium text-text-muted transition-colors hover:bg-accent/15 hover:text-accent"
									>Self / Home</button>
								</div>
							{/if}
						</div>
					{/if}
				{/each}
			</div>
		</div>
		</FadeIn>

		<!-- Development -->
		<FadeIn delay={100}>
		<DevelopmentSection
			rollId={id}
			{labs}
			bind:labDev
			bind:selfDev
			bind:devStages
			bind:autoPrompt={devAutoPrompt}
			defaultDate={shots.length > 0 ? (shots[shots.length - 1].date ?? '') : (roll?.date_loaded ?? '')}
			onchange={loadRollData}
		/>

		</FadeIn>

		<!-- Shots -->
		<FadeIn delay={150}>
		<div>
			<div class="mb-3 flex items-center justify-between">
				<div class="flex items-center gap-3">
					<h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">Shots</h2>
					{#if frameProgress}
						<div class="flex items-center gap-2">
							<span class="text-xs text-text-faint">{frameProgress.current}/{frameProgress.total}</span>
							<div class="h-1.5 w-24 overflow-hidden rounded-full bg-surface-overlay">
								<div
									class="h-full rounded-full transition-all duration-300 {frameProgress.current > frameProgress.total ? 'bg-red-400' : 'bg-accent'}"
									style="width: {Math.min((frameProgress.current / frameProgress.total) * 100, 100)}%"
								></div>
							</div>
						</div>
					{/if}
				</div>
				<div class="flex gap-2">
					<Button size="sm" variant="ghost" href="/quick-entry?roll={roll?.id}">Quick Entry</Button>
					<Button size="sm" onclick={openAddShotDialog}>+ Add Shot</Button>
				</div>
			</div>

			{#if showRollFullNudge}
				<div class="mb-3 flex flex-wrap items-end justify-between gap-3 rounded-lg border border-accent/30 bg-accent/10 px-4 py-3">
					<div class="flex flex-col gap-2">
						<div>
							<p class="text-sm font-medium text-accent">Roll complete</p>
							<p class="text-xs text-accent/70">
								All {frameProgress?.total} frames shot. When did you finish it?
							</p>
						</div>
						<div class="w-44">
							<DateInput label="Finished shooting" bind:value={finishDate} />
						</div>
					</div>
					<div class="flex items-center gap-2">
						<Button size="sm" variant="primary" onclick={() => updateStatus('shot', finishDate)}>Mark as Shot</Button>
						<button
							onclick={() => { rollFullDismissed = true; }}
							class="text-accent/60 hover:text-accent transition-colors text-lg leading-none px-1"
							aria-label="Dismiss"
						>&times;</button>
					</div>
				</div>
			{:else if frameProgress && shots.length > frameProgress.total}
				<div class="mb-3 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-400">
					More shots ({shots.length}) than the roll's frame count ({frameProgress.total}). This may indicate extra frames or a counting error.
				</div>
			{/if}

			{#if shots.length === 0}
				<p class="text-sm text-text-faint">No shots logged yet. Add your first shot to start tracking frames.</p>
			{:else}
				<div class="space-y-1.5">
					{#each shots as shot}
						{@const lensDisplay = getShotLensDisplay(shot.id)}
						<div class="group flex items-start justify-between rounded-lg border border-border bg-surface-raised px-4 py-2.5 transition-all duration-150 hover:border-accent/40">
							<div class="flex items-start gap-3">
								<span class="mt-0.5 inline-flex h-6 min-w-6 items-center justify-center rounded bg-accent/15 px-1.5 font-mono text-xs font-medium text-accent">
									{shot.frame_number}
								</span>
								<div>
									<div class="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-sm">
										{#if shot.aperture}
											<span class="text-text-muted">f/{shot.aperture}</span>
										{/if}
										{#if shot.shutter_speed}
											<span class="text-text-muted">{shot.shutter_speed}</span>
										{/if}
										{#if lensDisplay}
											<span class="text-text-faint">
												{lensDisplay.name}
												{#if lensDisplay.isDefault}
													<span class="text-text-faint/60 italic">(roll default)</span>
												{/if}
											</span>
										{/if}
										{#if shot.date}
											<span class="text-text-faint">{shot.date}</span>
										{/if}
										{#if shot.location}
											<span class="text-text-faint">{shot.location}</span>
										{/if}
									</div>
									{#if shot.notes}
										<p class="mt-0.5 text-xs text-text-faint">{shot.notes}</p>
									{/if}
								</div>
							</div>
							<div class="flex gap-1 opacity-0 transition-opacity group-hover:opacity-100">
								<Button size="sm" variant="ghost" onclick={() => openEditShotDialog(shot)}>Edit</Button>
								<Button size="sm" variant="ghost" onclick={() => (deletingShotId = shot.id)}>&times;</Button>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
		</FadeIn>
	</div>
{/if}

<!-- Add/Edit Shot Dialog -->
{#if showShotDialog}
	<Dialog open={true} title={editingShotId ? 'Edit Shot' : 'Add Shot'} onclose={() => { showShotDialog = false; resetShotForm(); }}>
		<div class="space-y-4">
			<div class="grid grid-cols-3 gap-3">
				<Input label="Frame #" bind:value={shotFrameNumber} placeholder="1" required />
				<Input label="Aperture (f/)" bind:value={shotAperture} placeholder="5.6" />
				<Input label="Shutter Speed" bind:value={shotShutterSpeed} placeholder="1/125" />
			</div>
			<div class="grid grid-cols-2 gap-3">
				<DateInput label="Date" bind:value={shotDate} />
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
			<div class="flex justify-end gap-2 pt-2">
				<Button variant="ghost" onclick={() => { showShotDialog = false; resetShotForm(); }}>Cancel</Button>
				{#if !editingShotId}
					<Button variant="ghost" onclick={handleSaveShotAndNext}>Save & Next</Button>
				{/if}
				<Button variant="primary" onclick={handleSaveShot}>
					{editingShotId ? 'Save' : 'Add Shot'}
				</Button>
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
		oncancel={() => { deletingShotId = null; }}
	/>
{/if}

<!-- Backward Status Move Confirmation -->
{#if pendingStatus}
	<ConfirmDialog
		open={true}
		title="Move Status Back"
		message={`Move status back to ${statusConfig[pendingStatus].label}? This will revert progress.`}
		confirmLabel="Move Back"
		onconfirm={() => { updateStatus(pendingStatus!); pendingStatus = null; }}
		oncancel={() => { pendingStatus = null; }}
	/>
{/if}
