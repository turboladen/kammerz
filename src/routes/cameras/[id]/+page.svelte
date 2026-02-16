<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import { getCamera, updateCamera, deleteCamera, listMaintenanceForCamera, createMaintenance, deleteMaintenance, listDistinctCameraBrands, listDistinctVendors, listDistinctMaintProviders } from '$lib/api/cameras';
	import { listDistinctLensBrands } from '$lib/api/lenses';
	import { listRollsForCamera } from '$lib/api/rolls';
	import type { Camera, CameraMaintenance, CameraMaintenanceInsert, RollWithDetails } from '$lib/types';

	const id = $derived(Number(page.params.id));

	let camera: Camera | undefined = $state();
	let maintenance: CameraMaintenance[] = $state([]);
	let rolls: RollWithDetails[] = $state([]);
	let loading = $state(true);
	let editing = $state(false);
	let showMaintenanceDialog = $state(false);
	let showDeleteConfirm = $state(false);
	let deletingMaintenanceId: number | null = $state(null);

	// Autocomplete options
	let brandOptions: string[] = $state([]);
	let vendorOptions: string[] = $state([]);
	let maintProviderOptions: string[] = $state([]);

	// Edit form state (populated when editing starts)
	let editBrand = $state('');
	let editModel = $state('');
	let editPrefix = $state('');
	let editFormat = $state('35mm');
	let editCameraType = $state('');
	let editSerialNumber = $state('');
	let editDatePurchased = $state('');
	let editPurchasedFrom = $state('');
	let editDateSold = $state('');
	let editNotes = $state('');

	// Maintenance form state
	let maintType = $state('CLA');
	let maintDoneBy = $state('');
	let maintDateDone = $state('');
	let maintCost = $state('');
	let maintNotes = $state('');
	let error = $state('');

	const formatOptions = [
		{ value: '35mm', label: '35mm' },
		{ value: 'medium format', label: 'Medium Format' },
		{ value: '6x4.5', label: 'Medium Format: 6x4.5' },
		{ value: '6x6', label: 'Medium Format: 6x6' },
		{ value: '6x7', label: 'Medium Format: 6x7' },
		{ value: '6x8', label: 'Medium Format: 6x8' },
		{ value: '6x9', label: 'Medium Format: 6x9' },
		{ value: 'large format', label: 'Large Format' },
		{ value: '4x5', label: 'Large Format: 4x5' },
		{ value: '5x7', label: 'Large Format: 5x7' },
		{ value: '8x10', label: 'Large Format: 8x10' },
		{ value: 'instant', label: 'Instant' }
	];

	const typeOptions = [
		{ value: '', label: 'Not specified' },
		{ value: 'SLR', label: 'SLR' },
		{ value: 'rangefinder', label: 'Rangefinder' },
		{ value: 'TLR', label: 'TLR' },
		{ value: 'view', label: 'View Camera' },
		{ value: 'point-and-shoot', label: 'Point & Shoot' },
		{ value: 'instant', label: 'Instant' }
	];

	const maintTypeOptions = [
		{ value: 'CLA', label: 'CLA (Clean, Lubricate, Adjust)' },
		{ value: 'repair', label: 'Repair' },
		{ value: 'cleaning', label: 'Cleaning' },
		{ value: 'modification', label: 'Modification' },
		{ value: 'other', label: 'Other' }
	];

	async function load() {
		try {
			const [cam, maint, r, camBrands, lensBrands, vendors, maintProviders] = await Promise.all([
				getCamera(id),
				listMaintenanceForCamera(id),
				listRollsForCamera(id),
				listDistinctCameraBrands(),
				listDistinctLensBrands(),
				listDistinctVendors(),
				listDistinctMaintProviders()
			]);
			camera = cam;
			maintenance = maint;
			rolls = r;
			brandOptions = [...new Set([...camBrands, ...lensBrands])].sort();
			vendorOptions = vendors;
			maintProviderOptions = maintProviders;
		} finally {
			loading = false;
		}
	}

	function startEditing() {
		if (!camera) return;
		editBrand = camera.brand;
		editModel = camera.model;
		editPrefix = camera.prefix ?? '';
		editFormat = camera.format;
		editCameraType = camera.camera_type ?? '';
		editSerialNumber = camera.serial_number ?? '';
		editDatePurchased = camera.date_purchased ?? '';
		editPurchasedFrom = camera.purchased_from ?? '';
		editDateSold = camera.date_sold ?? '';
		editNotes = camera.notes ?? '';
		editing = true;
	}

	async function saveEdit() {
		error = '';
		try {
			await updateCamera(id, {
				brand: editBrand,
				model: editModel,
				prefix: editPrefix || null,
				format: editFormat,
				camera_type: editCameraType || null,
				serial_number: editSerialNumber || null,
				date_purchased: editDatePurchased || null,
				purchased_from: editPurchasedFrom || null,
				date_sold: editDateSold || null,
				notes: editNotes || null
			});
			editing = false;
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleDelete() {
		showDeleteConfirm = true;
	}

	async function confirmDelete() {
		error = '';
		try {
			await deleteCamera(id);
			goto('/cameras');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function addMaintenance() {
		error = '';
		try {
			const record: CameraMaintenanceInsert = {
				camera_id: id,
				maintenance_type: maintType,
				done_by: maintDoneBy || null,
				date_done: maintDateDone || null,
				cost: maintCost ? parseFloat(maintCost) : null,
				notes: maintNotes || null
			};
			await createMaintenance(record);
			showMaintenanceDialog = false;
			maintType = 'CLA';
			maintDoneBy = '';
			maintDateDone = '';
			maintCost = '';
			maintNotes = '';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function removeMaintenance(maintId: number) {
		deletingMaintenanceId = maintId;
	}

	async function confirmRemoveMaintenance() {
		if (deletingMaintenanceId === null) return;
		error = '';
		try {
			await deleteMaintenance(deletingMaintenanceId);
			deletingMaintenanceId = null;
			await load();
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
{:else if !camera}
	<PageHeader title="Camera not found" />
	<div class="p-6">
		<Button href="/cameras">&larr; Back to cameras</Button>
	</div>
{:else if editing}
	<PageHeader title="Edit {camera.brand} {camera.model}" backHref="/cameras" backLabel="Cameras">
		<Button variant="ghost" onclick={() => (editing = false)}>Cancel</Button>
		<Button variant="primary" onclick={saveEdit}>Save</Button>
	</PageHeader>
	<div class="max-w-2xl p-6">
		<div class="space-y-4">
			<div class="grid grid-cols-2 gap-4">
				<ComboInput label="Brand" bind:value={editBrand} options={brandOptions} />
				<Input label="Model" bind:value={editModel} required />
			</div>
			<div class="grid grid-cols-2 gap-4">
				<Select label="Format" bind:value={editFormat} options={formatOptions} />
				<Select label="Type" bind:value={editCameraType} options={typeOptions} />
			</div>
			<div class="grid grid-cols-2 gap-4">
				<Input label="Prefix (Legacy ID)" bind:value={editPrefix} />
				<Input label="Serial Number" bind:value={editSerialNumber} />
			</div>
			<div class="grid grid-cols-2 gap-4">
				<Input label="Date Purchased" bind:value={editDatePurchased} type="date" />
				<ComboInput label="Purchased From" bind:value={editPurchasedFrom} options={vendorOptions} />
			</div>
			<Input label="Date Sold" bind:value={editDateSold} type="date" hint="Leave empty if you still own it" />
			<Textarea label="Notes" bind:value={editNotes} />
			{#if error}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
			{/if}
		</div>
	</div>
{:else}
	<PageHeader title="{camera.brand} {camera.model}" backHref="/cameras" backLabel="Cameras">
		<Button variant="ghost" onclick={startEditing}>Edit</Button>
		<Button variant="danger" onclick={handleDelete}>Delete</Button>
	</PageHeader>

	<div class="p-6">
		{#if error}
			<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}

		<!-- Camera Details -->
		<div class="mb-8 grid grid-cols-2 gap-x-8 gap-y-3 rounded-lg border border-border bg-surface-raised p-5">
			<div>
				<span class="text-xs text-text-muted">Format</span>
				<p class="text-sm">{camera.format}</p>
			</div>
			{#if camera.camera_type}
				<div>
					<span class="text-xs text-text-muted">Type</span>
					<p class="text-sm">{camera.camera_type}</p>
				</div>
			{/if}
			{#if camera.prefix}
				<div>
					<span class="text-xs text-text-muted">Prefix</span>
					<p class="font-mono text-sm">{camera.prefix}</p>
				</div>
			{/if}
			{#if camera.serial_number}
				<div>
					<span class="text-xs text-text-muted">Serial Number</span>
					<p class="text-sm">{camera.serial_number}</p>
				</div>
			{/if}
			{#if camera.date_purchased}
				<div>
					<span class="text-xs text-text-muted">Purchased</span>
					<p class="text-sm">{camera.date_purchased}{camera.purchased_from ? ` from ${camera.purchased_from}` : ''}</p>
				</div>
			{/if}
			{#if camera.date_sold}
				<div>
					<span class="text-xs text-text-muted">Sold</span>
					<p class="text-sm">{camera.date_sold}</p>
				</div>
			{/if}
			{#if camera.notes}
				<div class="col-span-2">
					<span class="text-xs text-text-muted">Notes</span>
					<p class="text-sm whitespace-pre-wrap">{camera.notes}</p>
				</div>
			{/if}
		</div>

		<!-- Maintenance History -->
		<div class="mb-8">
			<div class="mb-3 flex items-center justify-between">
				<h2 class="text-sm font-semibold text-text-muted">Maintenance History</h2>
				<Button size="sm" onclick={() => (showMaintenanceDialog = true)}>+ Add Record</Button>
			</div>
			{#if maintenance.length === 0}
				<p class="text-sm text-text-faint">No maintenance records yet.</p>
			{:else}
				<div class="space-y-2">
					{#each maintenance as record}
						<div class="flex items-start justify-between rounded-lg border border-border bg-surface-raised p-3">
							<div>
								<div class="flex items-center gap-2">
									<span class="rounded bg-accent/15 px-1.5 py-0.5 text-xs font-medium text-accent">{record.maintenance_type}</span>
									{#if record.date_done}
										<span class="text-xs text-text-muted">{record.date_done}</span>
									{/if}
								</div>
								{#if record.done_by}
									<p class="mt-1 text-sm">Done by: {record.done_by}</p>
								{/if}
								{#if record.cost}
									<p class="text-xs text-text-muted">${record.cost.toFixed(2)}</p>
								{/if}
								{#if record.notes}
									<p class="mt-1 text-sm text-text-muted">{record.notes}</p>
								{/if}
							</div>
							<Button size="sm" variant="ghost" onclick={() => removeMaintenance(record.id)}>&times;</Button>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Rolls shot with this camera -->
		<div>
			<h2 class="mb-3 text-sm font-semibold text-text-muted">Rolls ({rolls.length})</h2>
			{#if rolls.length === 0}
				<p class="text-sm text-text-faint">No rolls shot with this camera yet.</p>
			{:else}
				<div class="space-y-2">
					{#each rolls as roll}
						<a
							href="/rolls/{roll.id}"
							class="flex items-center justify-between rounded-lg border border-border bg-surface-raised p-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
						>
							<div class="flex items-center gap-3">
								<span class="font-mono text-sm">{roll.roll_id}</span>
								<Badge status={roll.status} />
								{#if roll.film_stock_brand}
									<span class="text-xs text-text-muted">{roll.film_stock_brand} {roll.film_stock_name}</span>
								{/if}
							</div>
						</a>
					{/each}
				</div>
			{/if}
		</div>
	</div>
{/if}

<!-- Add Maintenance Dialog -->
<Dialog bind:open={showMaintenanceDialog} title="Add Maintenance Record">
	<div class="space-y-4">
		<Select label="Type" bind:value={maintType} options={maintTypeOptions} />
		<ComboInput label="Done By" bind:value={maintDoneBy} placeholder="Garry's Camera Repair" options={maintProviderOptions} />
		<div class="grid grid-cols-2 gap-4">
			<Input label="Date" bind:value={maintDateDone} type="date" />
			<Input label="Cost ($)" bind:value={maintCost} type="number" step="0.01" placeholder="0.00" />
		</div>
		<Textarea label="Notes" bind:value={maintNotes} placeholder="What was done..." />
		<div class="flex justify-end gap-2 pt-2">
			<Button variant="ghost" onclick={() => (showMaintenanceDialog = false)}>Cancel</Button>
			<Button variant="primary" onclick={addMaintenance}>Add Record</Button>
		</div>
	</div>
</Dialog>

<!-- Delete Camera Confirmation -->
<ConfirmDialog
	bind:open={showDeleteConfirm}
	title="Delete Camera"
	message={camera ? `Permanently delete ${camera.brand} ${camera.model}? This will also remove all maintenance records for this camera.` : ''}
	confirmLabel="Delete Camera"
	onconfirm={confirmDelete}
	oncancel={() => {}}
/>

<!-- Delete Maintenance Confirmation -->
{#if deletingMaintenanceId !== null}
	<ConfirmDialog
		open={true}
		title="Delete Maintenance Record"
		message="Permanently delete this maintenance record?"
		confirmLabel="Delete Record"
		onconfirm={confirmRemoveMaintenance}
		oncancel={() => { deletingMaintenanceId = null; }}
	/>
{/if}
