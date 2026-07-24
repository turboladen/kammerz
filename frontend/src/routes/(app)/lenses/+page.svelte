<script lang="ts">
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import ListToolbar from '$lib/components/ui/ListToolbar.svelte';
	import GroupHeader from '$lib/components/ui/GroupHeader.svelte';
	import { Aperture } from 'lucide-svelte';
	import { listLenses, createLens, updateLens, deleteLens, listDistinctLensBrands } from '$lib/api/lenses';
	import { listCameras, listDistinctCameraBrands, listDistinctVendors } from '$lib/api/cameras';
	import { listLensMounts, createLensMount } from '$lib/api/lens-mounts';
	import { filterBySearch, groupItems, sortByString, sortByNumber, sortByDate } from '$lib/utils/list';
	import type { Camera, Lens, LensInsert, LensMount } from '$lib/types';
	import { lensDisplayName, buildMountOptions, NEW_MOUNT_OPTION } from '$lib/utils/lens';
	import { dateFieldError } from '$lib/utils/date';

	let lenses: Lens[] = $state([]);
	let loading = $state(true);
	let showAddDialog = $state(false);
	let editingLens: Lens | null = $state(null);
	let deletingLens: Lens | null = $state(null);
	let filterOwned = $state('all');
	let error = $state('');
	// In-flight guards: block a double-click from firing a mutation twice. `saving`
	// covers the add/edit dialogs (temporally exclusive); `savingMount` covers the
	// inline "+ New mount" create (a POST that would otherwise duplicate mount rows).
	let saving = $state(false);
	let savingMount = $state(false);

	// Toolbar state (?q= pre-filters the list, e.g. from a search result link)
	let searchQuery = $state(page.url.searchParams.get('q') ?? '');
	let groupBy = $state('brand');
	let sortBy = $state('brand-asc');

	// Autocomplete options
	let brandOptions: string[] = $state([]);
	let lensMounts: LensMount[] = $state([]);
	let vendorOptions: string[] = $state([]);
	let cameras: Camera[] = $state([]);

	const lensMountOptions = $derived(buildMountOptions(lensMounts));

	const mountNameById = $derived(Object.fromEntries(lensMounts.map((m) => [m.id, m.name])));

	const fixedMountIds = $derived(new Set(lensMounts.filter((m) => m.name === 'Fixed Lens').map((m) => m.id)));

	/** Maps lens ID → camera display name for lenses that are a camera's default (i.e. fixed) lens. */
	const fixedOnCamera = $derived(
		Object.fromEntries(
			cameras.filter((c) => c.default_lens_id != null).map((c) => [c.default_lens_id!, `${c.brand} ${c.model}`])
		) as Record<number, string>
	);

	// Pipeline: ownership filter → search → sort → group
	const afterOwnerFilter = $derived(
		filterOwned === 'all'
			? lenses
			: filterOwned === 'owned'
				? lenses.filter((l) => !l.date_sold)
				: lenses.filter((l) => l.date_sold)
	);

	const afterSearch = $derived(
		filterBySearch(afterOwnerFilter, searchQuery, (l) =>
			[lensDisplayName(l), l.brand, l.focal_length ?? '', l.max_aperture ?? '', l.serial_number ?? ''].join(' ')
		)
	);

	const afterSort = $derived.by(() => {
		switch (sortBy) {
			case 'brand-desc':
				return sortByString(afterSearch, (l) => lensDisplayName(l), 'desc');
			case 'focal-length-asc':
				return sortByNumber(
					afterSearch,
					(l) => {
						const n = parseFloat(l.focal_length ?? '');
						return isNaN(n) ? null : n;
					},
					'asc'
				);
			case 'focal-length-desc':
				return sortByNumber(
					afterSearch,
					(l) => {
						const n = parseFloat(l.focal_length ?? '');
						return isNaN(n) ? null : n;
					},
					'desc'
				);
			case 'date-purchased-desc':
				return sortByDate(afterSearch, (l) => l.date_purchased, 'desc');
			case 'date-purchased-asc':
				return sortByDate(afterSearch, (l) => l.date_purchased, 'asc');
			case 'date-added-desc':
				return sortByDate(afterSearch, (l) => l.created_at, 'desc');
			default:
				return sortByString(afterSearch, (l) => lensDisplayName(l), 'asc');
		}
	});

	const grouped = $derived.by(() => {
		if (groupBy === 'none') return groupItems(afterSort, () => '');
		if (groupBy === 'mount') return groupItems(afterSort, (l) => mountNameById[l.lens_mount_id] ?? 'Unknown');
		return groupItems(afterSort, (l) => l.brand);
	});

	const resultCount = $derived(afterSearch.length);

	const groupByOptions = [
		{ value: 'brand', label: 'Brand' },
		{ value: 'mount', label: 'Mount' },
		{ value: 'none', label: 'None' }
	];

	const sortOptions = [
		{ value: 'brand-asc', label: 'A\u2013Z' },
		{ value: 'brand-desc', label: 'Z\u2013A' },
		{ value: 'focal-length-asc', label: 'Focal Length \u2191' },
		{ value: 'focal-length-desc', label: 'Focal Length \u2193' },
		{ value: 'date-purchased-desc', label: 'Newest Purchase' },
		{ value: 'date-purchased-asc', label: 'Oldest Purchase' },
		{ value: 'date-added-desc', label: 'Recently Added' }
	];

	// Form state
	let brand = $state('');
	let lensMountId = $state('');
	let lensModel = $state('');
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
	// Both the Add and Edit dialogs bind these same vars, so one gate covers both.
	const dateError = $derived(dateFieldError(datePurchased) || dateFieldError(dateSold));

	// Inline mount creation (revealed when the mount Select picks "+ New mount…").
	// Both dialogs bind lensMountId, so this state is shared across add and edit.
	let newMountName = $state('');
	let newMountError = $state('');
	const creatingMount = $derived(lensMountId === NEW_MOUNT_OPTION);

	async function createMount() {
		if (savingMount) return;
		newMountError = '';
		const name = newMountName.trim();
		if (!name) {
			newMountError = 'Mount name is required.';
			return;
		}
		savingMount = true;
		try {
			const id = await createLensMount(name);
			lensMounts = await listLensMounts();
			lensMountId = String(id);
			newMountName = '';
		} catch (err) {
			newMountError = err instanceof Error ? err.message : String(err);
		} finally {
			savingMount = false;
		}
	}

	async function load() {
		try {
			const [l, lensBrands, camBrands, mounts, vendors, cams] = await Promise.all([
				listLenses(),
				listDistinctLensBrands(),
				listDistinctCameraBrands(),
				listLensMounts(),
				listDistinctVendors(),
				listCameras()
			]);
			lenses = l;
			brandOptions = [...new Set([...lensBrands, ...camBrands])].sort();
			lensMounts = mounts;
			vendorOptions = vendors;
			cameras = cams;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	function resetForm() {
		brand = '';
		lensMountId = '';
		lensModel = '';
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
		newMountName = '';
		newMountError = '';
	}

	function openAddDialog() {
		editingLens = null; // defensively enforce add/edit mutual exclusion (kammerz-f9pr)
		resetForm();
		error = '';
		showAddDialog = true;
	}

	function buildInsert(): LensInsert {
		return {
			brand,
			lens_mount_id: Number(lensMountId),
			lens_system: null,
			model: lensModel || null,
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
		if (saving) return;
		error = '';
		if (!brand.trim()) {
			error = 'Brand is required.';
			return;
		}
		if (!lensMountId) {
			error = 'Lens mount is required.';
			return;
		}
		// NaN defense-in-depth (kammerz-8cm8): the "+ New mount…" sentinel would serialize to
		// NaN as lens_mount_id (buildInsert does Number(lensMountId)). The Add button is already
		// disabled while `creatingMount`, so this guard is UI-unreachable today — the visible
		// hint under the mount Select is what tells the user to finish the sub-form. Kept as
		// belt-and-suspenders.
		if (creatingMount) {
			error = 'Finish creating the new mount, or pick an existing one.';
			return;
		}
		saving = true;
		try {
			await createLens(buildInsert());
			showAddDialog = false;
			resetForm();
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	function startEdit(lens: Lens) {
		showAddDialog = false; // defensively enforce add/edit mutual exclusion (kammerz-f9pr)
		error = '';
		editingLens = lens;
		brand = lens.brand;
		lensMountId = String(lens.lens_mount_id);
		lensModel = lens.model ?? '';
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
		if (saving) return;
		if (!editingLens) return;
		error = '';
		if (!brand.trim()) {
			error = 'Brand is required.';
			return;
		}
		if (!lensMountId) {
			error = 'Lens mount is required.';
			return;
		}
		// NaN defense-in-depth (kammerz-8cm8): see handleAdd — the "+ New mount…" sentinel would
		// serialize to NaN. Button already disabled while `creatingMount`; visible hint is the
		// user-facing message. Belt-and-suspenders.
		if (creatingMount) {
			error = 'Finish creating the new mount, or pick an existing one.';
			return;
		}
		saving = true;
		try {
			await updateLens(editingLens.id, buildInsert());
			editingLens = null;
			resetForm();
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	function handleDelete(lens: Lens) {
		deletingLens = lens;
	}

	async function confirmDelete() {
		if (!deletingLens) return;
		const lens = deletingLens;
		// Close the dialog before the request — a failure is reported via the
		// page error banner, and the dialog stays re-openable.
		deletingLens = null;
		error = '';
		try {
			await deleteLens(lens.id);
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="Lenses" description="Your lens collection">
	<Button variant="primary" onclick={openAddDialog}>+ Add Lens</Button>
</PageHeader>

<div class="p-6">
	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	<!-- Toolbar: search + group + sort -->
	<ListToolbar
		bind:searchQuery
		bind:groupBy
		bind:sortBy
		{groupByOptions}
		{sortOptions}
		{resultCount}
		totalCount={afterOwnerFilter.length}
		placeholder="Search lenses..."
	/>

	<div class="mb-4 flex gap-2">
		<Button variant={filterOwned === 'all' ? 'primary' : 'ghost'} size="sm" onclick={() => (filterOwned = 'all')}
			>All</Button
		>
		<Button variant={filterOwned === 'owned' ? 'primary' : 'ghost'} size="sm" onclick={() => (filterOwned = 'owned')}
			>Owned</Button
		>
		<Button variant={filterOwned === 'sold' ? 'primary' : 'ghost'} size="sm" onclick={() => (filterOwned = 'sold')}
			>Sold</Button
		>
	</div>

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if resultCount === 0 && lenses.length === 0}
		<EmptyState title="No Lenses" message="Add your first lens to get started.">
			{#snippet icon()}<Aperture size={24} strokeWidth={1.5} />{/snippet}
			<Button variant="primary" onclick={openAddDialog}>+ Add Lens</Button>
		</EmptyState>
	{:else if resultCount === 0}
		<p class="mt-6 text-center text-sm text-text-muted">
			{searchQuery ? `No lenses match your search.` : 'No lenses match the current filters.'}
		</p>
	{:else}
		{#each Object.entries(grouped) as [groupKey, groupLenses]}
			<GroupHeader label={groupKey} />
			<div class="mb-4 grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-2.5">
				{#each groupLenses as lens, i}
					<FadeIn delay={Math.min(i, 10) * 30}>
						<div
							class="group flex h-full flex-col rounded-lg border border-border bg-surface-raised px-3.5 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
						>
							<div class="flex items-start justify-between gap-2">
								<div class="min-w-0 flex-1">
									<div class="flex items-center gap-2">
										<span class="truncate text-sm font-semibold leading-snug">{lensDisplayName(lens)}</span>
										{#if lens.date_sold}
											<span class="shrink-0 rounded bg-red-500/15 px-1 py-0.5 text-[10px] text-red-400">Sold</span>
										{/if}
									</div>
									<div class="mt-1 text-xs text-text-muted">
										{#if fixedMountIds.has(lens.lens_mount_id)}
											<span class="text-accent/70"
												>Fixed{#if fixedOnCamera[lens.id]}{' '}on {fixedOnCamera[lens.id]}{/if}</span
											>
										{:else if mountNameById[lens.lens_mount_id]}
											<span>{mountNameById[lens.lens_mount_id]}</span>
										{/if}
										{#if lens.focal_length}
											<span class="mx-1 text-text-faint/60">·</span><span>{lens.focal_length}mm</span>
										{/if}
										{#if lens.max_aperture}
											<span class="mx-1 text-text-faint/60">·</span><span
												>f/{lens.max_aperture}{lens.min_aperture ? `–${lens.min_aperture}` : ''}</span
											>
										{/if}
									</div>
									{#if lens.filter_thread_front_mm || lens.serial_number}
										<div class="mt-1 text-[11px] text-text-faint">
											{#if lens.filter_thread_front_mm}
												<span>⌀{lens.filter_thread_front_mm}mm</span>
											{/if}
											{#if lens.serial_number}
												{#if lens.filter_thread_front_mm}<span class="mx-1 text-text-faint/40">·</span>{/if}
												<span class="font-mono">S/N {lens.serial_number}</span>
											{/if}
										</div>
									{/if}
								</div>
								<div
									class="flex shrink-0 gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100 pointer-coarse:opacity-100"
								>
									<Button size="sm" variant="ghost" onclick={() => startEdit(lens)}>Edit</Button>
									<Button size="sm" variant="ghost" onclick={() => handleDelete(lens)}>&times;</Button>
								</div>
							</div>
						</div>
					</FadeIn>
				{/each}
			</div>
		{/each}
	{/if}
</div>

<!-- Add Lens Dialog -->
<Dialog bind:open={showAddDialog} title="Add Lens" onclose={resetForm}>
	<div class="space-y-4">
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<ComboInput label="Brand/Manufacturer" bind:value={brand} placeholder="Minolta" options={brandOptions} />
			<Select label="Lens Mount" bind:value={lensMountId} options={lensMountOptions} />
		</div>
		{#if creatingMount}
			<div class="rounded-lg border border-border-subtle bg-surface px-3 py-3">
				<div class="flex items-end gap-2">
					<div class="flex-1">
						<Input label="New Mount Name" bind:value={newMountName} placeholder="Nikon F" spellcheck="false" />
					</div>
					<Button variant="primary" disabled={savingMount} onclick={createMount}>Create</Button>
					<Button
						variant="ghost"
						onclick={() => {
							lensMountId = '';
							newMountName = '';
							newMountError = '';
						}}>Cancel</Button
					>
				</div>
				{#if newMountError}
					<div class="mt-2 text-sm text-red-400">{newMountError}</div>
				{/if}
			</div>
		{/if}
		<Input label="Model" bind:value={lensModel} placeholder="MD Rokkor 50mm f/1.4" hint="Don't include the brand" />
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<Input label="Focal Length (mm)" bind:value={focalLength} placeholder="50 or 28-85" />
			<Input label="Max Aperture (f/)" bind:value={maxAperture} placeholder="1.4" />
		</div>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<Input label="Min Aperture (f/)" bind:value={minAperture} placeholder="22" />
			<Input label="Filter Thread Front (mm)" bind:value={filterFrontMm} type="number" placeholder="55" />
		</div>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<Input label="Filter Thread Rear (mm)" bind:value={filterRearMm} type="number" placeholder="" />
			<Input label="Serial Number" bind:value={serialNumber} />
		</div>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<Input type="date" label="Date Purchased" class="h-[38px]" bind:value={datePurchased} />
			<ComboInput label="Purchased From" bind:value={purchasedFrom} options={vendorOptions} />
		</div>
		<Input
			type="date"
			label="Date Sold"
			class="h-[38px]"
			bind:value={dateSold}
			hint="Leave empty if you still own it"
		/>
		<Textarea label="Notes" bind:value={notes} />
		{#if error}
			<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}
		{#if creatingMount}
			<p class="text-right text-xs text-text-faint">
				Create the new mount above (or pick an existing one) to continue.
			</p>
		{/if}
		<div class="flex justify-end gap-2 pt-2">
			<Button
				variant="ghost"
				onclick={() => {
					showAddDialog = false;
					resetForm();
				}}>Cancel</Button
			>
			<Button variant="primary" disabled={saving || !!dateError || creatingMount} onclick={handleAdd}>Add Lens</Button>
		</div>
	</div>
</Dialog>

<!-- Edit Lens Dialog (reuses same form fields) -->
{#if editingLens}
	<Dialog
		open={true}
		title="Edit Lens"
		onclose={() => {
			editingLens = null;
			resetForm();
		}}
	>
		<div class="space-y-4">
			{#if fixedMountIds.has(editingLens.lens_mount_id)}
				{@const cameraName = fixedOnCamera[editingLens.id]}
				<div class="rounded-lg border border-accent/25 bg-accent/5 px-3 py-2 text-sm text-accent">
					Fixed lens{#if cameraName}{' '}on <span class="font-semibold">{cameraName}</span>{/if}
				</div>
			{/if}
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<ComboInput label="Brand/Manufacturer" bind:value={brand} options={brandOptions} />
				<Select label="Lens Mount" bind:value={lensMountId} options={lensMountOptions} />
			</div>
			{#if creatingMount}
				<div class="rounded-lg border border-border-subtle bg-surface px-3 py-3">
					<div class="flex items-end gap-2">
						<div class="flex-1">
							<Input label="New Mount Name" bind:value={newMountName} placeholder="Nikon F" spellcheck="false" />
						</div>
						<Button variant="primary" disabled={savingMount} onclick={createMount}>Create</Button>
						<Button
							variant="ghost"
							onclick={() => {
								lensMountId = '';
								newMountName = '';
								newMountError = '';
							}}>Cancel</Button
						>
					</div>
					{#if newMountError}
						<div class="mt-2 text-sm text-red-400">{newMountError}</div>
					{/if}
				</div>
			{/if}
			<Input label="Model" bind:value={lensModel} hint="Don't include the brand" />
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<Input label="Focal Length (mm)" bind:value={focalLength} />
				<Input label="Max Aperture (f/)" bind:value={maxAperture} />
			</div>
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<Input label="Min Aperture (f/)" bind:value={minAperture} />
				<Input label="Filter Thread Front (mm)" bind:value={filterFrontMm} type="number" />
			</div>
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<Input label="Filter Thread Rear (mm)" bind:value={filterRearMm} type="number" />
				<Input label="Serial Number" bind:value={serialNumber} />
			</div>
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<Input type="date" label="Date Purchased" class="h-[38px]" bind:value={datePurchased} />
				<ComboInput label="Purchased From" bind:value={purchasedFrom} options={vendorOptions} />
			</div>
			<Input
				type="date"
				label="Date Sold"
				class="h-[38px]"
				bind:value={dateSold}
				hint="Leave empty if you still own it"
			/>
			<Textarea label="Notes" bind:value={notes} />
			{#if error}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
			{/if}
			{#if creatingMount}
				<p class="text-right text-xs text-text-faint">
					Create the new mount above (or pick an existing one) to continue.
				</p>
			{/if}
			<div class="flex justify-end gap-2 pt-2">
				<Button
					variant="ghost"
					onclick={() => {
						editingLens = null;
						resetForm();
					}}>Cancel</Button
				>
				<Button variant="primary" disabled={saving || !!dateError || creatingMount} onclick={handleEdit}>Save</Button>
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
		oncancel={() => {
			deletingLens = null;
		}}
	/>
{/if}
