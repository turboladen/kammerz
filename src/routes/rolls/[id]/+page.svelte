<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import DevelopmentSection from '$lib/components/rolls/DevelopmentSection.svelte';
	import { getRoll, updateRoll, deleteRoll } from '$lib/api/rolls';
	import { listCameras, getLensesForCamera } from '$lib/api/cameras';
	import { listLenses } from '$lib/api/lenses';
	import { listShotsForRoll, createShot, updateShot, deleteShot, getLensesForShot, suggestNextFrame } from '$lib/api/shots';
	import { getLabDevForRoll, getSelfDevForRoll, listDevStages } from '$lib/api/development';
	import { listLabs } from '$lib/api/labs';
	import { lensDisplayName } from '$lib/utils/lens';
	import type { RollWithDetails, Camera, Lens, Shot, Lab, DevelopmentLab, DevelopmentSelf, DevStage, RollStatus } from '$lib/types';

	const id = $derived(Number(page.params.id));

	let roll: RollWithDetails | undefined = $state();
	let cameras: Camera[] = $state([]);
	let loading = $state(true);
	let assignCameraId = $state('');
	let showDeleteConfirm = $state(false);
	let error = $state('');

	// Shot state
	let shots: Shot[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let cameraLensIds: number[] = $state([]);
	let shotLensMap: Record<number, number[]> = $state({});
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
	let shotLensIds: number[] = $state([]);
	let shotError = $state('');

	// Development state (shared with DevelopmentSection component)
	let labs: Lab[] = $state([]);
	let labDev: DevelopmentLab | null = $state(null);
	let selfDev: DevelopmentSelf | null = $state(null);
	let devStages: DevStage[] = $state([]);
	let devAutoPrompt: 'lab' | 'self' | null = $state(null);

	const statusProgression: RollStatus[] = [
		'loaded', 'shooting', 'shot', 'at-lab', 'developing', 'developed', 'scanned', 'archived'
	];

	const cameraOptions = $derived([
		{ value: '', label: 'Not assigned' },
		...cameras.map((c) => ({ value: String(c.id), label: `${c.brand} ${c.model}` }))
	]);

	// Lenses sorted: camera-linked first, then others. Only owned (not sold).
	const availableLenses = $derived.by(() => {
		const owned = allLenses.filter((l) => !l.date_sold);
		const linked = owned.filter((l) => cameraLensIds.includes(l.id));
		const other = owned.filter((l) => !cameraLensIds.includes(l.id));
		return [...linked, ...other];
	});

	// Frame progress
	const frameProgress = $derived.by(() => {
		if (!roll?.frame_count) return null;
		return { current: shots.length, total: roll.frame_count };
	});

	async function load() {
		try {
			const [r, cams, s, lenses, labsList, ld, sd] = await Promise.all([
				getRoll(id),
				listCameras(),
				listShotsForRoll(id),
				listLenses(),
				listLabs(),
				getLabDevForRoll(id),
				getSelfDevForRoll(id)
			]);
			roll = r;
			cameras = cams;
			shots = s;
			allLenses = lenses;
			labs = labsList;
			labDev = ld;
			selfDev = sd;
			assignCameraId = roll?.camera_id?.toString() ?? '';

			// Load dev stages if self-development exists
			if (sd) {
				devStages = await listDevStages(sd.id);
			} else {
				devStages = [];
			}

			// Load camera-lens associations if camera is set
			if (roll?.camera_id) {
				cameraLensIds = await getLensesForCamera(roll.camera_id);
			} else {
				cameraLensIds = [];
			}

			// Batch-load lens IDs per shot
			const map: Record<number, number[]> = {};
			await Promise.all(
				s.map(async (shot) => {
					map[shot.id] = await getLensesForShot(shot.id);
				})
			);
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
		shotLensIds = [];
		shotError = '';
	}

	async function openAddShotDialog() {
		resetShotForm();
		editingShotId = null;
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
		shotLensIds = shotLensMap[shot.id] ?? [];
		shotError = '';
		showShotDialog = true;
	}

	async function handleSaveShot() {
		shotError = '';
		if (!shotFrameNumber.trim()) {
			shotError = 'Frame number is required.';
			return;
		}
		try {
			if (editingShotId) {
				await updateShot(editingShotId, {
					frame_number: shotFrameNumber.trim(),
					aperture: shotAperture || null,
					shutter_speed: shotShutterSpeed || null,
					date: shotDate || null,
					location: shotLocation || null,
					notes: shotNotes || null,
					lens_ids: shotLensIds
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
					lens_ids: shotLensIds
				});
			}
			showShotDialog = false;
			resetShotForm();
			await load();
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

	function toggleShotLens(lensId: number) {
		if (shotLensIds.includes(lensId)) {
			shotLensIds = shotLensIds.filter((id) => id !== lensId);
		} else {
			shotLensIds = [...shotLensIds, lensId];
		}
	}

	function getLensNamesForShot(shotId: number): string {
		const ids = shotLensMap[shotId] ?? [];
		return ids
			.map((lid) => allLenses.find((l) => l.id === lid))
			.filter(Boolean)
			.map((l) => lensDisplayName(l!))
			.join(', ');
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

	async function assignCamera() {
		error = '';
		try {
			await updateRoll(id, { camera_id: assignCameraId ? Number(assignCameraId) : null });
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
		<div class="mb-6 rounded-lg border border-border bg-surface-raised p-5">
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
					{#if roll.date_fuzzy}
						<p class="mt-1 text-xs italic text-text-faint">{roll.date_fuzzy}</p>
					{/if}
					{#if roll.notes}
						<p class="mt-2 text-sm text-text-muted whitespace-pre-wrap">{roll.notes}</p>
					{/if}
				</div>
			</div>
		</div>

		<!-- Camera Assignment -->
		{#if !roll.camera_id}
			<div class="mb-6 rounded-lg border border-amber-500/30 bg-amber-500/10 p-4">
				<p class="mb-2 text-sm font-medium text-amber-400">No camera assigned</p>
				<div class="flex items-end gap-2">
					<div class="flex-1">
						<Select label="Assign Camera" bind:value={assignCameraId} options={cameraOptions} />
					</div>
					<Button variant="primary" size="sm" onclick={assignCamera} disabled={!assignCameraId}>Assign</Button>
				</div>
			</div>
		{:else}
			<div class="mb-6 flex items-center gap-2">
				<Select label="Camera" bind:value={assignCameraId} options={cameraOptions} />
				<div class="pt-5">
					<Button size="sm" onclick={assignCamera}>Update</Button>
				</div>
			</div>
		{/if}

		<!-- Status Progression -->
		<div class="mb-6">
			<h2 class="mb-3 text-sm font-semibold text-text-muted">Status</h2>
			<div class="flex flex-wrap gap-2">
				{#each statusProgression as status}
					<button
						onclick={() => updateStatus(status)}
						class="rounded-lg px-3 py-1.5 text-xs font-medium transition-colors
							{roll.status === status
							? 'bg-accent text-surface'
							: 'bg-surface-overlay text-text-muted hover:text-text'}"
					>
						{status}
					</button>
				{/each}
			</div>
		</div>

		<!-- Development -->
		<DevelopmentSection
			rollId={id}
			{labs}
			bind:labDev
			bind:selfDev
			bind:devStages
			bind:autoPrompt={devAutoPrompt}
			onchange={load}
		/>

		<!-- Shots -->
		<div>
			<div class="mb-3 flex items-center justify-between">
				<div class="flex items-center gap-3">
					<h2 class="text-sm font-semibold text-text-muted">Shots</h2>
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
				<Button size="sm" onclick={openAddShotDialog}>+ Add Shot</Button>
			</div>

			{#if frameProgress && shots.length > frameProgress.total}
				<div class="mb-3 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-400">
					More shots ({shots.length}) than the roll's frame count ({frameProgress.total}). This may indicate extra frames or a counting error.
				</div>
			{/if}

			{#if shots.length === 0}
				<p class="text-sm text-text-faint">No shots logged yet. Add your first shot to start tracking frames.</p>
			{:else}
				<div class="space-y-1.5">
					{#each shots as shot}
						{@const lensNames = getLensNamesForShot(shot.id)}
						<div class="group flex items-start justify-between rounded-lg border border-border bg-surface-raised px-4 py-2.5 transition-all duration-150 hover:border-accent/30">
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
										{#if lensNames}
											<span class="text-text-faint">{lensNames}</span>
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
				<Input label="Date" bind:value={shotDate} type="date" />
				<Input label="Location" bind:value={shotLocation} placeholder="Central Park" />
			</div>

			<!-- Lens Selection (checkboxes) -->
			{#if availableLenses.length > 0}
				<div>
					<span class="mb-1.5 block text-xs font-medium text-text-muted">Lens</span>
					<div class="max-h-36 space-y-1 overflow-y-auto rounded-lg border border-border bg-surface p-2">
						{#each availableLenses as lens}
							{@const isLinked = cameraLensIds.includes(lens.id)}
							<label class="flex cursor-pointer items-center gap-2 rounded px-2 py-1 text-sm transition-colors hover:bg-surface-overlay">
								<input
									type="checkbox"
									checked={shotLensIds.includes(lens.id)}
									onchange={() => toggleShotLens(lens.id)}
									class="rounded border-border accent-accent"
								/>
								<span>{lensDisplayName(lens)}</span>
								{#if isLinked}
									<span class="text-xs text-text-faint">(camera)</span>
								{/if}
							</label>
						{/each}
					</div>
				</div>
			{/if}

			<Textarea label="Notes" bind:value={shotNotes} placeholder="Any notes about this shot..." />

			{#if shotError}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{shotError}</div>
			{/if}
			<div class="flex justify-end gap-2 pt-2">
				<Button variant="ghost" onclick={() => { showShotDialog = false; resetShotForm(); }}>Cancel</Button>
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
