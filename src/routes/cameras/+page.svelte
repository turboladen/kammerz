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
	import { Camera as CameraIcon } from 'lucide-svelte';
	import { listCameras, createCamera, createCameraWithLens, deleteCamera, listDistinctCameraBrands, listDistinctVendors } from '$lib/api/cameras';
	import { listDistinctLensBrands } from '$lib/api/lenses';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import { buildMountOptions } from '$lib/utils/lens';
	import type { Camera, CameraInsert, LensMount } from '$lib/types';

	let cameras: Camera[] = $state([]);
	let lensMounts: LensMount[] = $state([]);
	let loading = $state(true);
	let showAddDialog = $state(false);
	let filterOwned = $state('all');
	let error: string = $state('');

	// Autocomplete options
	let brandOptions: string[] = $state([]);
	let vendorOptions: string[] = $state([]);

	const filtered = $derived(
		filterOwned === 'all'
			? cameras
			: filterOwned === 'owned'
				? cameras.filter((c) => !c.date_sold)
				: cameras.filter((c) => c.date_sold)
	);

	const mountNameById = $derived(
		Object.fromEntries(lensMounts.map((m) => [m.id, m.name]))
	);

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

	// Inline fixed-lens fields
	let lensNameOnLens = $state('');
	let lensFocalLength = $state('');
	let lensMaxAperture = $state('');

	const isFixedLens = $derived(
		lensMountId
			? lensMounts.find((m) => m.id === Number(lensMountId))?.name === 'Fixed Lens'
			: false
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
		lensNameOnLens = '';
		lensFocalLength = '';
		lensMaxAperture = '';
	}

	async function handleAdd() {
		error = '';
		try {
			const camera: CameraInsert = {
				brand,
				model,
				prefix: prefix || null,
				format,
				lens_mount_id: Number(lensMountId),
				default_lens_id: null,
				camera_type: cameraType || null,
				serial_number: serialNumber || null,
				date_purchased: datePurchased || null,
				purchased_from: purchasedFrom || null,
				date_sold: dateSold || null,
				notes: notes || null
			};

			if (isFixedLens && (lensNameOnLens || lensFocalLength || lensMaxAperture)) {
				await createCameraWithLens({
					camera,
					lens_name_on_lens: lensNameOnLens || null,
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

	async function handleDelete(id: number) {
		await deleteCamera(id);
		await load();
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

	<div class="mb-4 flex gap-2">
		<Button
			variant={filterOwned === 'all' ? 'primary' : 'ghost'}
			size="sm"
			onclick={() => (filterOwned = 'all')}
		>All</Button>
		<Button
			variant={filterOwned === 'owned' ? 'primary' : 'ghost'}
			size="sm"
			onclick={() => (filterOwned = 'owned')}
		>Owned</Button>
		<Button
			variant={filterOwned === 'sold' ? 'primary' : 'ghost'}
			size="sm"
			onclick={() => (filterOwned = 'sold')}
		>Sold</Button>
	</div>

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if filtered.length === 0}
		<EmptyState title="No Cameras" message="Add your first camera to get started.">
			{#snippet icon()}<CameraIcon size={24} strokeWidth={1.5} />{/snippet}
			<Button variant="primary" onclick={() => (showAddDialog = true)}>+ Add Camera</Button>
		</EmptyState>
	{:else}
		<div class="grid gap-3">
			{#each filtered as camera, i}
				<FadeIn delay={Math.min(i, 10) * 30}>
					<a
						href="/cameras/{camera.id}"
						class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised p-4 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
					>
						<div>
							<div class="flex items-center gap-2">
								<span class="font-semibold">{camera.brand} {camera.model}</span>
								{#if camera.prefix}
									<span class="rounded bg-surface-overlay px-1.5 py-0.5 font-mono text-xs text-text-muted">{camera.prefix}</span>
								{/if}
								{#if camera.date_sold}
									<span class="rounded bg-red-500/15 px-1.5 py-0.5 text-xs text-red-400">Sold</span>
								{/if}
							</div>
							<div class="mt-1 flex gap-3 text-xs text-text-muted">
								<span>{camera.format}</span>
								{#if mountNameById[camera.lens_mount_id]}
									<span>{mountNameById[camera.lens_mount_id]}</span>
								{/if}
								{#if camera.camera_type}
									<span>{camera.camera_type}</span>
								{/if}
								{#if camera.serial_number}
									<span>S/N: {camera.serial_number}</span>
								{/if}
							</div>
						</div>
						<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">View &rarr;</span>
					</a>
				</FadeIn>
			{/each}
		</div>
	{/if}
</div>

<!-- Add Camera Dialog -->
<Dialog bind:open={showAddDialog} title="Add Camera">
	<div class="space-y-4">
		<div class="grid grid-cols-2 gap-4">
			<ComboInput label="Brand" bind:value={brand} placeholder="Minolta" options={brandOptions} />
			<Input label="Model" bind:value={model} placeholder="XD-7" spellcheck="false" />
		</div>
		<div class="grid grid-cols-3 gap-4">
			<Select label="Format" bind:value={format} options={formatOptions} />
			<Select label="Lens Mount" bind:value={lensMountId} options={lensMountOptions} />
			<Select label="Type" bind:value={cameraType} options={typeOptions} />
		</div>
		{#if isFixedLens}
			<div class="border-t border-border-subtle pt-4">
				<h3 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
					Fixed Lens Details
					<div class="flex-1 border-b border-border-subtle"></div>
				</h3>
				<div class="space-y-4">
					<Input label="Name on Lens" bind:value={lensNameOnLens} placeholder="Rokkor 75mm 1:3.5" hint="Brand, mount, and purchase info will match the camera" />
					<div class="grid grid-cols-2 gap-4">
						<Input label="Focal Length (mm)" bind:value={lensFocalLength} placeholder="75" />
						<Input label="Max Aperture (f/)" bind:value={lensMaxAperture} placeholder="3.5" />
					</div>
				</div>
			</div>
		{/if}
		<div class="grid grid-cols-2 gap-4">
			<Input label="Prefix (Legacy ID)" bind:value={prefix} placeholder="MD7" hint="Optional, for legacy roll IDs" />
			<Input label="Serial Number" bind:value={serialNumber} placeholder="1234567" />
		</div>
		<div class="grid grid-cols-2 gap-4">
			<DateInput label="Date Purchased" bind:value={datePurchased} />
			<ComboInput label="Purchased From" bind:value={purchasedFrom} placeholder="eBay, KEH, etc." options={vendorOptions} />
		</div>
		<DateInput label="Date Sold" bind:value={dateSold} hint="Leave empty if you still own it" />
		<Textarea label="Notes" bind:value={notes} placeholder="Any notes about this camera..." />
		{#if error}
			<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}
		<div class="flex justify-end gap-2 pt-2">
			<Button variant="ghost" onclick={() => (showAddDialog = false)}>Cancel</Button>
			<Button variant="primary" onclick={handleAdd}>Add Camera</Button>
		</div>
	</div>
</Dialog>
