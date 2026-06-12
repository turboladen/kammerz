<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import DateInput from '$lib/components/ui/DateInput.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import ListToolbar from '$lib/components/ui/ListToolbar.svelte';
	import { dateFieldError } from '$lib/utils/date';
	import GroupHeader from '$lib/components/ui/GroupHeader.svelte';
	import { Camera as CameraIcon } from 'lucide-svelte';
	import {
		listCameras,
		createCamera,
		createCameraWithLens,
		listDistinctCameraBrands,
		listDistinctVendors
	} from '$lib/api/cameras';
	import { listDistinctLensBrands } from '$lib/api/lenses';
	import { listLensMounts, createLensMount } from '$lib/api/lens-mounts';
	import { buildMountOptions, NEW_MOUNT_OPTION } from '$lib/utils/lens';
	import { filterBySearch, groupItems, sortByString, sortByDate } from '$lib/utils/list';
	import type { Camera, CameraFormat, CameraType, CameraInsert, LensMount } from '$lib/types';

	let cameras: Camera[] = $state([]);
	let lensMounts: LensMount[] = $state([]);
	let loading = $state(true);
	let showAddDialog = $state(false);
	let filterOwned = $state('all');
	let error: string = $state('');

	// Toolbar state
	let searchQuery = $state('');
	let groupBy = $state('brand');
	let sortBy = $state('brand-asc');

	// Autocomplete options
	let brandOptions: string[] = $state([]);
	let vendorOptions: string[] = $state([]);

	const mountNameById = $derived(Object.fromEntries(lensMounts.map((m) => [m.id, m.name])));

	// Pipeline: ownership filter → search → sort → group
	const afterOwnerFilter = $derived(
		filterOwned === 'all'
			? cameras
			: filterOwned === 'owned'
				? cameras.filter((c) => !c.date_sold)
				: cameras.filter((c) => c.date_sold)
	);

	const afterSearch = $derived(
		filterBySearch(afterOwnerFilter, searchQuery, (c) =>
			[c.brand, c.model, c.camera_type ?? '', c.serial_number ?? ''].join(' ')
		)
	);

	const afterSort = $derived.by(() => {
		switch (sortBy) {
			case 'brand-desc':
				return sortByString(afterSearch, (c) => `${c.brand} ${c.model}`, 'desc');
			case 'date-purchased-desc':
				return sortByDate(afterSearch, (c) => c.date_purchased, 'desc');
			case 'date-purchased-asc':
				return sortByDate(afterSearch, (c) => c.date_purchased, 'asc');
			case 'date-added-desc':
				return sortByDate(afterSearch, (c) => c.created_at, 'desc');
			case 'format-asc':
				return sortByString(afterSearch, (c) => c.format, 'asc');
			default:
				return sortByString(afterSearch, (c) => `${c.brand} ${c.model}`, 'asc');
		}
	});

	const grouped = $derived.by(() => {
		if (groupBy === 'none') return groupItems(afterSort, () => '');
		if (groupBy === 'format') return groupItems(afterSort, (c) => c.format);
		if (groupBy === 'type') return groupItems(afterSort, (c) => c.camera_type ?? 'Unspecified');
		if (groupBy === 'mount') return groupItems(afterSort, (c) => mountNameById[c.lens_mount_id] ?? 'Unknown');
		return groupItems(afterSort, (c) => c.brand);
	});

	const resultCount = $derived(afterSearch.length);

	const groupByOptions = [
		{ value: 'brand', label: 'Brand' },
		{ value: 'mount', label: 'Mount' },
		{ value: 'format', label: 'Format' },
		{ value: 'type', label: 'Type' },
		{ value: 'none', label: 'None' }
	];

	const sortOptions = [
		{ value: 'brand-asc', label: 'A\u2013Z' },
		{ value: 'brand-desc', label: 'Z\u2013A' },
		{ value: 'date-purchased-desc', label: 'Newest Purchase' },
		{ value: 'date-purchased-asc', label: 'Oldest Purchase' },
		{ value: 'date-added-desc', label: 'Recently Added' },
		{ value: 'format-asc', label: 'Format' }
	];

	// Form state
	let brand = $state('');
	let model = $state('');
	let prefix = $state('');
	let format = $state('35mm');
	let lensMountId = $state('');
	let cameraType = $state('');
	let serialNumber = $state('');
	let datePurchased = $state('');
	let purchasedFrom = $state('');
	let dateSold = $state('');
	let notes = $state('');
	const addDateError = $derived(dateFieldError(datePurchased) || dateFieldError(dateSold));

	// Inline mount creation (revealed when the mount Select picks "+ New mount…")
	let newMountName = $state('');
	let newMountError = $state('');
	const creatingMount = $derived(lensMountId === NEW_MOUNT_OPTION);

	// Inline fixed-lens fields
	let lensModel = $state('');
	let lensFocalLength = $state('');
	let lensMaxAperture = $state('');

	const isFixedLens = $derived(
		lensMountId ? lensMounts.find((m) => m.id === Number(lensMountId))?.name === 'Fixed Lens' : false
	);

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

	const lensMountOptions = $derived(buildMountOptions(lensMounts));

	const typeOptions = [
		{ value: '', label: 'Not specified' },
		{ value: 'SLR', label: 'SLR' },
		{ value: 'rangefinder', label: 'Rangefinder' },
		{ value: 'TLR', label: 'TLR' },
		{ value: 'view', label: 'View Camera' },
		{ value: 'point-and-shoot', label: 'Point & Shoot' },
		{ value: 'box', label: 'Box Camera' },
		{ value: 'instant', label: 'Instant' }
	];

	async function load() {
		try {
			const [cams, camBrands, lensBrands, vendors, mounts] = await Promise.all([
				listCameras(),
				listDistinctCameraBrands(),
				listDistinctLensBrands(),
				listDistinctVendors(),
				listLensMounts()
			]);
			cameras = cams;
			brandOptions = [...new Set([...camBrands, ...lensBrands])].sort();
			vendorOptions = vendors;
			lensMounts = mounts;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	function resetForm() {
		brand = '';
		model = '';
		prefix = '';
		format = '35mm';
		lensMountId = '';
		cameraType = '';
		serialNumber = '';
		datePurchased = '';
		purchasedFrom = '';
		dateSold = '';
		notes = '';
		lensModel = '';
		lensFocalLength = '';
		lensMaxAperture = '';
		newMountName = '';
		newMountError = '';
		error = '';
	}

	async function createMount() {
		newMountError = '';
		const name = newMountName.trim();
		if (!name) {
			newMountError = 'Mount name is required.';
			return;
		}
		try {
			const id = await createLensMount(name);
			lensMounts = await listLensMounts();
			lensMountId = String(id);
			newMountName = '';
		} catch (err) {
			newMountError = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleAdd() {
		error = '';
		if (!brand.trim()) {
			error = 'Brand is required.';
			return;
		}
		if (!model.trim()) {
			error = 'Model is required.';
			return;
		}
		if (!lensMountId) {
			error = 'Lens mount is required.';
			return;
		}
		if (isFixedLens && !lensFocalLength.trim()) {
			error = 'Focal length is required for fixed-lens cameras.';
			return;
		}
		try {
			const camera: CameraInsert = {
				brand,
				model,
				prefix: prefix || null,
				format: format as CameraFormat,
				lens_mount_id: Number(lensMountId),
				default_lens_id: null,
				camera_type: (cameraType || null) as CameraType | null,
				serial_number: serialNumber || null,
				date_purchased: datePurchased || null,
				purchased_from: purchasedFrom || null,
				date_sold: dateSold || null,
				notes: notes || null
			};

			if (isFixedLens) {
				await createCameraWithLens({
					camera,
					lens_model: lensModel || null,
					lens_focal_length: lensFocalLength || null,
					lens_max_aperture: lensMaxAperture || null
				});
			} else {
				await createCamera(camera);
			}

			showAddDialog = false;
			resetForm();
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="Cameras" description="Your camera collection">
	<Button variant="primary" onclick={() => (showAddDialog = true)}>+ Add Camera</Button>
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
		placeholder="Search cameras..."
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
	{:else if resultCount === 0 && cameras.length === 0}
		<EmptyState title="No Cameras" message="Add your first camera to get started.">
			{#snippet icon()}<CameraIcon size={24} strokeWidth={1.5} />{/snippet}
			<Button variant="primary" onclick={() => (showAddDialog = true)}>+ Add Camera</Button>
		</EmptyState>
	{:else if resultCount === 0}
		<p class="mt-6 text-center text-sm text-text-muted">
			{searchQuery ? `No cameras match your search.` : 'No cameras match the current filters.'}
		</p>
	{:else}
		{#each Object.entries(grouped) as [groupKey, groupCameras]}
			<GroupHeader label={groupKey} />
			<div class="mb-4 grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-2.5">
				{#each groupCameras as camera, i}
					<FadeIn delay={Math.min(i, 10) * 30}>
						<a
							href="/cameras/{camera.id}"
							class="group flex h-full flex-col rounded-lg border border-border bg-surface-raised px-3.5 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
						>
							<div class="flex items-center gap-2">
								<span class="text-sm font-semibold leading-snug">{camera.brand} {camera.model}</span>
								{#if camera.prefix}
									<span class="shrink-0 rounded bg-surface-overlay px-1.5 py-0.5 font-mono text-[10px] text-text-muted"
										>{camera.prefix}</span
									>
								{/if}
							</div>
							<div class="mt-1 text-xs text-text-muted">
								<span>{camera.format}</span>
								{#if mountNameById[camera.lens_mount_id]}
									<span class="mx-1 text-text-faint/60">·</span><span>{mountNameById[camera.lens_mount_id]}</span>
								{/if}
								{#if camera.camera_type}
									<span class="mx-1 text-text-faint/60">·</span><span>{camera.camera_type}</span>
								{/if}
							</div>
							{#if camera.serial_number || camera.date_sold}
								<div class="mt-1 flex items-center gap-2 text-[11px] text-text-faint">
									{#if camera.serial_number}
										<span class="font-mono">S/N {camera.serial_number}</span>
									{/if}
									{#if camera.date_sold}
										<span class="rounded bg-red-500/15 px-1 py-0.5 text-[10px] text-red-400">Sold</span>
									{/if}
								</div>
							{/if}
						</a>
					</FadeIn>
				{/each}
			</div>
		{/each}
	{/if}
</div>

<!-- Add Camera Dialog -->
<Dialog bind:open={showAddDialog} title="Add Camera" onclose={resetForm}>
	<div class="space-y-4">
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<ComboInput label="Brand" bind:value={brand} placeholder="Minolta" options={brandOptions} />
			<Input label="Model" bind:value={model} placeholder="XD-7" spellcheck="false" />
		</div>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
			<Select label="Format" bind:value={format} options={formatOptions} />
			<Select label="Lens Mount" bind:value={lensMountId} options={lensMountOptions} />
			<Select label="Type" bind:value={cameraType} options={typeOptions} />
		</div>
		{#if creatingMount}
			<div class="rounded-lg border border-border-subtle bg-surface px-3 py-3">
				<div class="flex items-end gap-2">
					<div class="flex-1">
						<Input label="New Mount Name" bind:value={newMountName} placeholder="Nikon F" spellcheck="false" />
					</div>
					<Button variant="primary" onclick={createMount}>Create</Button>
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
		{#if isFixedLens}
			<div class="border-t border-border-subtle pt-4">
				<h3 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
					Fixed Lens Details
					<div class="flex-1 border-b border-border-subtle"></div>
				</h3>
				<div class="space-y-4">
					<Input
						label="Model"
						bind:value={lensModel}
						placeholder="Rokkor 75mm f/3.5"
						hint="Brand, mount, and purchase info will match the camera"
					/>
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
						<Input label="Focal Length (mm)" bind:value={lensFocalLength} placeholder="75" />
						<Input label="Max Aperture (f/)" bind:value={lensMaxAperture} placeholder="3.5" />
					</div>
				</div>
			</div>
		{/if}
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<Input label="Prefix (Legacy ID)" bind:value={prefix} placeholder="MD7" hint="Optional, for legacy roll IDs" />
			<Input label="Serial Number" bind:value={serialNumber} placeholder="1234567" />
		</div>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<DateInput label="Date Purchased" bind:value={datePurchased} />
			<ComboInput
				label="Purchased From"
				bind:value={purchasedFrom}
				placeholder="eBay, KEH, etc."
				options={vendorOptions}
			/>
		</div>
		<DateInput label="Date Sold" bind:value={dateSold} hint="Leave empty if you still own it" />
		<Textarea label="Notes" bind:value={notes} placeholder="Any notes about this camera..." />
		{#if error}
			<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}
		<div class="flex justify-end gap-2 pt-2">
			<Button
				variant="ghost"
				onclick={() => {
					showAddDialog = false;
					resetForm();
				}}>Cancel</Button
			>
			<Button variant="primary" disabled={!!addDateError || creatingMount} onclick={handleAdd}>Add Camera</Button>
		</div>
	</div>
</Dialog>
