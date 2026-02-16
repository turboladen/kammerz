<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import { listLenses, createLens, updateLens, deleteLens, listDistinctLensBrands, listDistinctLensSystems } from '$lib/db/lenses';
	import { listDistinctCameraBrands, listDistinctVendors } from '$lib/db/cameras';
	import type { Lens, LensInsert } from '$lib/types';

	let lenses: Lens[] = $state([]);
	let loading = $state(true);
	let showAddDialog = $state(false);
	let editingLens: Lens | null = $state(null);
	let deletingLens: Lens | null = $state(null);

	// Autocomplete options
	let brandOptions: string[] = $state([]);
	let lensSystemOptions: string[] = $state([]);
	let vendorOptions: string[] = $state([]);

	// Form state
	let brand = $state('');
	let lensSystem = $state('');
	let nameOnLens = $state('');
	let focalLength = $state('');
	let maxAperture = $state('');
	let minAperture = $state('');
	let filterFrontMm = $state('');
	let filterRearMm = $state('');
	let serialNumber = $state('');
	let datePurchased = $state('');
	let purchasedFrom = $state('');
	let dateSold = $state('');
	let notes = $state('');

	async function load() {
		try {
			const [l, lensBrands, camBrands, systems, vendors] = await Promise.all([
				listLenses(),
				listDistinctLensBrands(),
				listDistinctCameraBrands(),
				listDistinctLensSystems(),
				listDistinctVendors()
			]);
			lenses = l;
			brandOptions = [...new Set([...lensBrands, ...camBrands])].sort();
			lensSystemOptions = systems;
			vendorOptions = vendors;
		} finally {
			loading = false;
		}
	}

	function resetForm() {
		brand = '';
		lensSystem = '';
		nameOnLens = '';
		focalLength = '';
		maxAperture = '';
		minAperture = '';
		filterFrontMm = '';
		filterRearMm = '';
		serialNumber = '';
		datePurchased = '';
		purchasedFrom = '';
		dateSold = '';
		notes = '';
	}

	function buildInsert(): LensInsert {
		return {
			brand,
			lens_system: lensSystem || null,
			name_on_lens: nameOnLens || null,
			focal_length: focalLength || null,
			max_aperture: maxAperture || null,
			min_aperture: minAperture || null,
			filter_thread_front_mm: filterFrontMm ? parseInt(filterFrontMm) : null,
			filter_thread_rear_mm: filterRearMm ? parseInt(filterRearMm) : null,
			serial_number: serialNumber || null,
			date_purchased: datePurchased || null,
			purchased_from: purchasedFrom || null,
			date_sold: dateSold || null,
			notes: notes || null
		};
	}

	async function handleAdd() {
		await createLens(buildInsert());
		showAddDialog = false;
		resetForm();
		await load();
	}

	function startEdit(lens: Lens) {
		editingLens = lens;
		brand = lens.brand;
		lensSystem = lens.lens_system ?? '';
		nameOnLens = lens.name_on_lens ?? '';
		focalLength = lens.focal_length ?? '';
		maxAperture = lens.max_aperture ?? '';
		minAperture = lens.min_aperture ?? '';
		filterFrontMm = lens.filter_thread_front_mm?.toString() ?? '';
		filterRearMm = lens.filter_thread_rear_mm?.toString() ?? '';
		serialNumber = lens.serial_number ?? '';
		datePurchased = lens.date_purchased ?? '';
		purchasedFrom = lens.purchased_from ?? '';
		dateSold = lens.date_sold ?? '';
		notes = lens.notes ?? '';
	}

	async function handleEdit() {
		if (!editingLens) return;
		await updateLens(editingLens.id, buildInsert());
		editingLens = null;
		resetForm();
		await load();
	}

	function handleDelete(lens: Lens) {
		deletingLens = lens;
	}

	async function confirmDelete() {
		if (!deletingLens) return;
		await deleteLens(deletingLens.id);
		deletingLens = null;
		await load();
	}

	$effect(() => {
		load();
	});

	function lensDisplayName(lens: Lens): string {
		if (lens.name_on_lens) return lens.name_on_lens;
		const parts = [lens.brand];
		if (lens.focal_length) parts.push(`${lens.focal_length}mm`);
		if (lens.max_aperture) parts.push(`f/${lens.max_aperture}`);
		return parts.join(' ');
	}
</script>

<PageHeader title="Lenses" description="Your lens collection">
	<Button variant="primary" onclick={() => { resetForm(); showAddDialog = true; }}>+ Add Lens</Button>
</PageHeader>

<div class="p-6">
	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if lenses.length === 0}
		<EmptyState message="No lenses yet. Add your first lens to get started.">
			<Button variant="primary" onclick={() => (showAddDialog = true)}>+ Add Lens</Button>
		</EmptyState>
	{:else}
		<div class="grid gap-3">
			{#each lenses as lens}
				<div class="group flex items-center justify-between rounded-xl border border-border bg-surface-raised p-4">
					<div>
						<div class="flex items-center gap-2">
							<span class="font-semibold">{lensDisplayName(lens)}</span>
							{#if lens.date_sold}
								<span class="rounded bg-red-500/15 px-1.5 py-0.5 text-xs text-red-400">Sold</span>
							{/if}
						</div>
						<div class="mt-1 flex flex-wrap gap-3 text-xs text-text-muted">
							{#if lens.lens_system}
								<span>{lens.lens_system} mount</span>
							{/if}
							{#if lens.focal_length}
								<span>{lens.focal_length}mm</span>
							{/if}
							{#if lens.max_aperture}
								<span>f/{lens.max_aperture}{lens.min_aperture ? ` - f/${lens.min_aperture}` : ''}</span>
							{/if}
							{#if lens.filter_thread_front_mm}
								<span>{lens.filter_thread_front_mm}mm filter</span>
							{/if}
						</div>
					</div>
					<div class="flex gap-1 opacity-0 transition-opacity group-hover:opacity-100">
						<Button size="sm" variant="ghost" onclick={() => startEdit(lens)}>Edit</Button>
						<Button size="sm" variant="ghost" onclick={() => handleDelete(lens)}>&times;</Button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Add/Edit Lens Dialog -->
<Dialog bind:open={showAddDialog} title="Add Lens">
	<div class="space-y-4">
		<div class="grid grid-cols-2 gap-4">
			<ComboInput label="Brand/Manufacturer" bind:value={brand} placeholder="Minolta" options={brandOptions} />
			<ComboInput label="Lens System (Mount)" bind:value={lensSystem} placeholder="Minolta MD" options={lensSystemOptions} />
		</div>
		<Input label="Name on Lens" bind:value={nameOnLens} placeholder="MD Rokkor 50mm 1:1.4" />
		<div class="grid grid-cols-2 gap-4">
			<Input label="Focal Length (mm)" bind:value={focalLength} placeholder="50 or 28-85" />
			<Input label="Max Aperture (f/)" bind:value={maxAperture} placeholder="1.4" />
		</div>
		<div class="grid grid-cols-2 gap-4">
			<Input label="Min Aperture (f/)" bind:value={minAperture} placeholder="22" />
			<Input label="Filter Thread Front (mm)" bind:value={filterFrontMm} type="number" placeholder="55" />
		</div>
		<div class="grid grid-cols-2 gap-4">
			<Input label="Filter Thread Rear (mm)" bind:value={filterRearMm} type="number" placeholder="" />
			<Input label="Serial Number" bind:value={serialNumber} />
		</div>
		<div class="grid grid-cols-2 gap-4">
			<Input label="Date Purchased" bind:value={datePurchased} type="date" />
			<ComboInput label="Purchased From" bind:value={purchasedFrom} options={vendorOptions} />
		</div>
		<Input label="Date Sold" bind:value={dateSold} type="date" />
		<Textarea label="Notes" bind:value={notes} />
		<div class="flex justify-end gap-2 pt-2">
			<Button variant="ghost" onclick={() => (showAddDialog = false)}>Cancel</Button>
			<Button variant="primary" onclick={handleAdd}>Add Lens</Button>
		</div>
	</div>
</Dialog>

<!-- Edit Lens Dialog (reuses same form fields) -->
{#if editingLens}
	<Dialog open={true} title="Edit Lens" onclose={() => { editingLens = null; resetForm(); }}>
		<div class="space-y-4">
			<div class="grid grid-cols-2 gap-4">
				<ComboInput label="Brand/Manufacturer" bind:value={brand} options={brandOptions} />
				<ComboInput label="Lens System (Mount)" bind:value={lensSystem} options={lensSystemOptions} />
			</div>
			<Input label="Name on Lens" bind:value={nameOnLens} />
			<div class="grid grid-cols-2 gap-4">
				<Input label="Focal Length (mm)" bind:value={focalLength} />
				<Input label="Max Aperture (f/)" bind:value={maxAperture} />
			</div>
			<div class="grid grid-cols-2 gap-4">
				<Input label="Min Aperture (f/)" bind:value={minAperture} />
				<Input label="Filter Thread Front (mm)" bind:value={filterFrontMm} type="number" />
			</div>
			<div class="grid grid-cols-2 gap-4">
				<Input label="Filter Thread Rear (mm)" bind:value={filterRearMm} type="number" />
				<Input label="Serial Number" bind:value={serialNumber} />
			</div>
			<div class="grid grid-cols-2 gap-4">
				<Input label="Date Purchased" bind:value={datePurchased} type="date" />
				<ComboInput label="Purchased From" bind:value={purchasedFrom} options={vendorOptions} />
			</div>
			<Input label="Date Sold" bind:value={dateSold} type="date" />
			<Textarea label="Notes" bind:value={notes} />
			<div class="flex justify-end gap-2 pt-2">
				<Button variant="ghost" onclick={() => { editingLens = null; resetForm(); }}>Cancel</Button>
				<Button variant="primary" onclick={handleEdit}>Save</Button>
			</div>
		</div>
	</Dialog>
{/if}

{#if deletingLens}
	<ConfirmDialog
		open={true}
		title="Delete Lens"
		message={`Permanently delete ${lensDisplayName(deletingLens)}?`}
		confirmLabel="Delete Lens"
		onconfirm={confirmDelete}
		oncancel={() => { deletingLens = null; }}
	/>
{/if}
