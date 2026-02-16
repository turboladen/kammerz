<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import { getRoll, updateRoll, deleteRoll } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import type { RollWithDetails, Camera, RollStatus } from '$lib/types';

	const id = $derived(Number(page.params.id));

	let roll: RollWithDetails | undefined = $state();
	let cameras: Camera[] = $state([]);
	let loading = $state(true);
	let assignCameraId = $state('');
	let showDeleteConfirm = $state(false);
	let error = $state('');

	const statusProgression: RollStatus[] = [
		'loaded', 'shooting', 'shot', 'at-lab', 'developing', 'developed', 'scanned', 'archived'
	];

	const cameraOptions = $derived([
		{ value: '', label: 'Not assigned' },
		...cameras.map((c) => ({ value: String(c.id), label: `${c.brand} ${c.model}` }))
	]);

	async function load() {
		try {
			const [r, cams] = await Promise.all([getRoll(id), listCameras()]);
			roll = r;
			cameras = cams;
			assignCameraId = roll?.camera_id?.toString() ?? '';
		} finally {
			loading = false;
		}
	}

	async function updateStatus(status: RollStatus) {
		error = '';
		try {
			await updateRoll(id, { status });
			await load();
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

		<!-- Placeholder for shots (Phase 3) -->
		<div class="rounded-lg border border-border-subtle bg-surface-raised p-5">
			<h2 class="mb-2 text-sm font-semibold text-text-muted">Shots</h2>
			<p class="text-sm text-text-faint">Shot entry will be added in the next phase.</p>
		</div>
	</div>
{/if}

<ConfirmDialog
	bind:open={showDeleteConfirm}
	title="Delete Roll"
	message={roll ? `Permanently delete roll ${roll.roll_id}? This cannot be undone.` : ''}
	confirmLabel="Delete Roll"
	onconfirm={confirmDelete}
	oncancel={() => {}}
/>
