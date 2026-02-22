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
	import { getRoll, updateRoll, deleteRoll } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import { listFilmStocks } from '$lib/api/film-stocks';
	import { listLenses } from '$lib/api/lenses';
	import { listShotsForRoll, createShot, updateShot, deleteShot, getLensesForShot, getLensesForRollShots, suggestNextFrame } from '$lib/api/shots';
	import { getLabDevForRoll, getSelfDevForRoll, listDevStages } from '$lib/api/development';
	import { listLabs } from '$lib/api/labs';
	import { lensDisplayName, buildLensOptions } from '$lib/utils/lens';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import { statusOrder, statusConfig } from '$lib/utils/status';
	import type { RollWithDetails, Camera, FilmStock, Lens, Shot, Lab, DevelopmentLab, DevelopmentSelf, DevStage, RollStatus, LensMount } from '$lib/types';

	const id = $derived(Number(page.params.id));

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

	// Status backward-move confirmation
	let pendingStatus: RollStatus | null = $state(null);
	const currentStatusIdx = $derived(roll ? statusOrder.indexOf(roll.status as RollStatus) : -1);

	// Roll-full nudge state
	let rollFullDismissed = $state(false);
	const showRollFullNudge = $derived(
		roll?.status === 'shooting' &&
		frameProgress !== null &&
		shots.length >= frameProgress.total &&
		!rollFullDismissed
	);

	// Dev-status nudge: suggest status change when dev record exists but status hasn't caught up
	let devNudgeDismissed = $state(false);
	const devStatusNudge = $derived.by(() => {
		if (devNudgeDismissed || !roll) return null;
		const currentIdx = statusOrder.indexOf(roll.status as RollStatus);
		if (labDev && currentIdx < statusOrder.indexOf('at-lab')) {
			return { target: 'at-lab' as RollStatus, label: 'At Lab', reason: 'Lab development added' };
		}
		if (selfDev && currentIdx < statusOrder.indexOf('developed')) {
			return { target: 'developed' as RollStatus, label: 'Developed', reason: 'Self development logged' };
		}
		return null;
	});

	const cameraOptions = $derived([
		{ value: '', label: 'Not assigned' },
		...cameras.map((c) => ({ value: String(c.id), label: `${c.brand} ${c.model}` }))
	]);

	const selectedCamera = $derived(
		roll?.camera_id ? cameras.find((c) => c.id === roll.camera_id) ?? null : null
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
		'large format': '4x5', '4x5': '4x5', '5x7': '5x7', '8x10': '8x10'
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

	// Hint for medium format variable-back cameras (120 film has no fixed exposure_count)
	const editFrameCountHint = $derived.by(() => {
		const matchingFormat = editSelectedCamera
			? cameraFormatToStockFormat[editSelectedCamera.format]
			: null;
		if (matchingFormat === '120' && !editFrameCount) {
			return '120 film: 6\u00d74.5=15 \u00b7 6\u00d76=12 \u00b7 6\u00d77=10 \u00b7 6\u00d78=9 \u00b7 6\u00d79=8';
		}
		return undefined;
	});

	// Shot-level lens dropdown options (uses the saved camera, not the edit form camera)
	const shotLensOptions = $derived(buildLensOptions(allLenses, selectedCamera, 'No lens', lensMounts));

	// Frame progress
	const frameProgress = $derived.by(() => {
		if (!roll?.frame_count) return null;
		return { current: shots.length, total: roll.frame_count };
	});

	async function load() {
		try {
			const [r, cams, stocks, s, lenses, labsList, ld, sd, mounts] = await Promise.all([
				getRoll(id),
				listCameras(),
				listFilmStocks(),
				listShotsForRoll(id),
				listLenses(),
				listLabs(),
				getLabDevForRoll(id),
				getSelfDevForRoll(id),
				listLensMounts()
			]);
			roll = r;
			rollFullDismissed = false;
			devNudgeDismissed = false;
			cameras = cams;
			filmStocks = stocks;
			shots = s;
			allLenses = lenses;
			labs = labsList;
			lensMounts = mounts;
			labDev = ld;
			selfDev = sd;

			// Load dev stages if self-development exists
			if (sd) {
				devStages = await listDevStages(sd.id);
			} else {
				devStages = [];
			}

			// Batch-load all shot-lens associations in a single query
			const pairs = await getLensesForRollShots(id);
			const map: Record<number, number[]> = {};
			for (const [shotId, lensId] of pairs) {
				(map[shotId] ??= []).push(lensId);
			}
			shotLensMap = map;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
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
			await load();
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
			await load();
			// Reset per-shot fields but keep session defaults (date, location, lens)
			const nextFrame = await suggestNextFrame(id);
			shotFrameNumber = nextFrame;
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
			await load();
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
			const defaultLens = allLenses.find((l) => l.id === roll.lens_id);
			if (defaultLens) {
				return { name: lensDisplayName(defaultLens), isDefault: true };
			}
		}
		return null;
	}

	async function updateStatus(status: RollStatus) {
		error = '';
		try {
			await updateRoll(id, { status });
			await load();
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
		const currentIdx = statusOrder.indexOf(roll.status as RollStatus);
		const targetIdx = statusOrder.indexOf(status);
		if (targetIdx < currentIdx) {
			// Backward move — ask for confirmation
			pendingStatus = status;
		} else {
			updateStatus(status);
		}
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
				push_pull: editPushPull || null,
				notes: editNotes || null
			});
			editingRoll = false;
			await load();
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

	$effect(() => {
		load();
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
	<PageHeader title="Roll {roll.roll_id}" backHref="/rolls" backLabel="Rolls">
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
			<div class="flex items-center gap-[2px]">
				{#each statusOrder as status, idx}
					{@const isFirst = idx === 0}
					{@const isLast = idx === statusOrder.length - 1}
					{@const clipPath = isFirst
						? 'polygon(0 0, calc(100% - 8px) 0, 100% 50%, calc(100% - 8px) 100%, 0 100%)'
						: isLast
							? 'polygon(0 0, 100% 0, 100% 100%, 0 100%, 8px 50%)'
							: 'polygon(0 0, calc(100% - 8px) 0, 100% 50%, calc(100% - 8px) 100%, 0 100%, 8px 50%)'}
					{@const isSkipped = idx < currentStatusIdx && (
						(status === 'at-lab' && !labDev) ||
						(status === 'developing' && !selfDev)
					)}
					<button
						onclick={() => handleStatusClick(status)}
						style="clip-path: {clipPath}"
						class="py-1.5 text-xs font-medium transition-colors
							{isFirst ? 'pl-3 pr-4' : isLast ? 'pl-4 pr-3' : 'px-4'}
							{roll.status === status
							? 'bg-accent text-surface'
							: isSkipped
								? 'bg-surface-overlay/60 text-text-faint'
								: idx < currentStatusIdx
									? 'bg-accent/10 text-accent/70 hover:bg-accent/20'
									: 'bg-surface-overlay text-text-muted hover:text-text'}"
					>
						{statusConfig[status].label}
					</button>
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
			onchange={load}
		/>

		{#if devStatusNudge}
			<div class="mb-6 flex items-center justify-between rounded-lg border border-accent/30 bg-accent/10 px-4 py-3">
				<div>
					<p class="text-sm font-medium text-accent">{devStatusNudge.reason}</p>
					<p class="text-xs text-accent/70">
						Move status to {devStatusNudge.label}?
					</p>
				</div>
				<div class="flex items-center gap-2">
					<Button size="sm" variant="primary" onclick={() => updateStatus(devStatusNudge.target)}>Move to {devStatusNudge.label}</Button>
					<button
						onclick={() => { devNudgeDismissed = true; }}
						class="text-accent/60 hover:text-accent transition-colors text-lg leading-none px-1"
						aria-label="Dismiss"
					>&times;</button>
				</div>
			</div>
		{/if}
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
				<div class="mb-3 flex items-center justify-between rounded-lg border border-accent/30 bg-accent/10 px-4 py-3">
					<div>
						<p class="text-sm font-medium text-accent">Roll complete</p>
						<p class="text-xs text-accent/70">
							All {frameProgress?.total} frames shot. Ready to mark as done?
						</p>
					</div>
					<div class="flex items-center gap-2">
						<Button size="sm" variant="primary" onclick={() => updateStatus('shot')}>Mark as Shot</Button>
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
