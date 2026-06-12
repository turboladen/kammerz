<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import DateInput from '$lib/components/ui/DateInput.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import { getCamera, updateCamera, deleteCamera, listMaintenanceForCamera, createMaintenance, deleteMaintenance, listDistinctCameraBrands, listDistinctVendors, listDistinctMaintProviders, getLensesForCamera, linkLensToCamera, unlinkLensFromCamera } from '$lib/api/cameras';
	import { listDistinctLensBrands, listLenses } from '$lib/api/lenses';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import { listRollsForCamera } from '$lib/api/rolls';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import { lensDisplayName, buildMountOptions } from '$lib/utils/lens';
	import { dateFieldError } from '$lib/utils/date';
	import type { Camera, CameraFormat, CameraType, MaintenanceType, CameraMaintenance, CameraMaintenanceInsert, Lens, LensMount, RollWithDetails } from '$lib/types';
	import { Trash2 } from 'lucide-svelte';

	const id = $derived(Number(page.params.id));

	// Back navigation: read ?from= param to determine where we came from
	const cameraBackRoutes: Record<string, { href: string; label: string }> = {
		search: { href: '/search', label: 'Search' }
	};
	const cameraFromParam = $derived(page.url.searchParams.get('from'));
	const cameraBackNav = $derived(cameraFromParam && cameraBackRoutes[cameraFromParam] ? cameraBackRoutes[cameraFromParam] : { href: '/cameras', label: 'Cameras' });

	let camera: Camera | undefined = $state();
	let maintenance: CameraMaintenance[] = $state([]);
	let rolls: RollWithDetails[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let lensMounts: LensMount[] = $state([]);
	let linkedLensIds: number[] = $state([]);
	let showLinkLensDialog = $state(false);
	let linkLensId = $state('');
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
	let editLensMountId = $state('');
	let editCameraType = $state('');
	let editSerialNumber = $state('');
	let editDatePurchased = $state('');
	let editPurchasedFrom = $state('');
	let editDateSold = $state('');
	let editNotes = $state('');
	const editDateError = $derived(dateFieldError(editDatePurchased) || dateFieldError(editDateSold));

	// Maintenance form state
	let maintType = $state('CLA');
	let maintDoneBy = $state('');
	let maintDateDone = $state('');
	let maintCost = $state('');
	let maintNotes = $state('');
	const maintDateError = $derived(dateFieldError(maintDateDone));
	let error = $state('');

	const lensMountOptions = $derived(buildMountOptions(lensMounts));

	const mountNameById = $derived(
		Object.fromEntries(lensMounts.map((m) => [m.id, m.name]))
	);

	const isFixedLens = $derived(
		camera ? mountNameById[camera.lens_mount_id] === 'Fixed Lens' : false
	);

	const linkedLenses = $derived(allLenses.filter((l) => linkedLensIds.includes(l.id)));
	const unlinkedLenses = $derived(allLenses.filter((l) => !linkedLensIds.includes(l.id) && !l.date_sold));
	const linkLensOptions = $derived.by(() => {
		const compatible = unlinkedLenses.filter((l) => l.lens_mount_id === camera?.lens_mount_id);
		const rest = unlinkedLenses.filter((l) => l.lens_mount_id !== camera?.lens_mount_id);
		const options: { value: string; label: string; disabled?: boolean }[] = [
			{ value: '', label: 'Select a lens...' }
		];
		for (const l of compatible) options.push({ value: String(l.id), label: lensDisplayName(l) });
		if (compatible.length > 0 && rest.length > 0) {
			options.push({ value: '__divider__', label: '── Other mounts ──', disabled: true });
		}
		for (const l of rest) options.push({ value: String(l.id), label: lensDisplayName(l) });
		return options;
	});

	// Default lens dropdown options (from linked lenses only)
	const defaultLensOptions = $derived([
		{ value: '', label: 'No default' },
		...linkedLenses.map((l) => ({ value: String(l.id), label: lensDisplayName(l) }))
	]);

	// Track default lens selection for the dropdown
	let defaultLensId = $state('');

	// Default lens display name for view mode
	const defaultLensDisplay = $derived.by(() => {
		const defLensId = camera?.default_lens_id;
		if (!defLensId) return null;
		const lens = linkedLenses.find((l) => l.id === defLensId);
		return lens ? lensDisplayName(lens) : null;
	});

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
		{ value: 'box', label: 'Box Camera' },
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
			const [cam, maint, r, lenses, camLensIds, camBrands, lensBrands, vendors, maintProviders, mounts] = await Promise.all([
				getCamera(id),
				listMaintenanceForCamera(id),
				listRollsForCamera(id),
				listLenses(),
				getLensesForCamera(id),
				listDistinctCameraBrands(),
				listDistinctLensBrands(),
				listDistinctVendors(),
				listDistinctMaintProviders(),
				listLensMounts()
			]);
			camera = cam ?? undefined;
			maintenance = maint;
			rolls = r;
			allLenses = lenses;
			linkedLensIds = camLensIds;
			brandOptions = [...new Set([...camBrands, ...lensBrands])].sort();
			vendorOptions = vendors;
			maintProviderOptions = maintProviders;
			lensMounts = mounts;
			defaultLensId = String(cam?.default_lens_id ?? '');
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
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
		editLensMountId = String(camera.lens_mount_id);
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
				format: editFormat as CameraFormat,
				lens_mount_id: Number(editLensMountId),
				camera_type: (editCameraType || null) as CameraType | null,
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
		// ConfirmDialog no longer closes itself on confirm — the parent owns
		// closing, so reset the bound state before the request.
		showDeleteConfirm = false;
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
				maintenance_type: maintType as MaintenanceType,
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
		const maintId = deletingMaintenanceId;
		// Close the dialog before the request — a failure is reported via the
		// page error banner, and the dialog stays re-openable.
		deletingMaintenanceId = null;
		error = '';
		try {
			await deleteMaintenance(maintId);
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleLinkLens() {
		if (!linkLensId) return;
		error = '';
		try {
			const lensIdToLink = Number(linkLensId);
			await linkLensToCamera(id, lensIdToLink);
			showLinkLensDialog = false;

			// Auto-set as default if this is the first linked lens
			const newLinkedIds = await getLensesForCamera(id);
			if (newLinkedIds.length === 1) {
				await updateCamera(id, { default_lens_id: lensIdToLink });
			}

			linkLensId = '';
			linkedLensIds = newLinkedIds;
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleUnlinkLens(lensId: number) {
		error = '';
		try {
			await unlinkLensFromCamera(id, lensId);
			// If unlinking the default lens, clear it
			if (camera?.default_lens_id === lensId) {
				await updateCamera(id, { default_lens_id: null });
			}
			linkedLensIds = await getLensesForCamera(id);
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleDefaultLensChange() {
		const newId = defaultLensId ? Number(defaultLensId) : null;
		if (camera && newId !== camera.default_lens_id) {
			error = '';
			try {
				await updateCamera(id, { default_lens_id: newId });
				camera = { ...camera, default_lens_id: newId };
			} catch (err) {
				error = err instanceof Error ? err.message : String(err);
			}
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
	<PageHeader title="Edit {camera.brand} {camera.model}" backHref={cameraBackNav.href} backLabel={cameraBackNav.label}>
		<Button variant="ghost" onclick={() => (editing = false)}>Cancel</Button>
		<Button variant="primary" disabled={!!editDateError} onclick={saveEdit}>Save</Button>
	</PageHeader>
	<div class="max-w-2xl p-6">
		<div class="space-y-4">
			<div class="grid grid-cols-2 gap-4">
				<ComboInput label="Brand" bind:value={editBrand} options={brandOptions} />
				<Input label="Model" bind:value={editModel} required spellcheck="false" />
			</div>
			<div class="grid grid-cols-3 gap-4">
				<Select label="Format" bind:value={editFormat} options={formatOptions} />
				{#if isFixedLens}
					<div>
						<span class="mb-1.5 block text-xs font-medium text-text-muted">Lens Mount</span>
						<div class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text-muted">Fixed Lens</div>
					</div>
				{:else}
					<Select label="Lens Mount" bind:value={editLensMountId} options={lensMountOptions} />
				{/if}
				<Select label="Type" bind:value={editCameraType} options={typeOptions} />
			</div>
			<div class="grid grid-cols-2 gap-4">
				<Input label="Prefix (Legacy ID)" bind:value={editPrefix} />
				<Input label="Serial Number" bind:value={editSerialNumber} />
			</div>
			<div class="grid grid-cols-2 gap-4">
				<DateInput label="Date Purchased" bind:value={editDatePurchased} />
				<ComboInput label="Purchased From" bind:value={editPurchasedFrom} options={vendorOptions} />
			</div>
			<DateInput label="Date Sold" bind:value={editDateSold} hint="Leave empty if you still own it" />
			<Textarea label="Notes" bind:value={editNotes} />
			{#if error}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
			{/if}
		</div>
	</div>
{:else}
	<PageHeader title="{camera.brand} {camera.model}" backHref={cameraBackNav.href} backLabel={cameraBackNav.label}>
		<Button variant="ghost" onclick={startEditing}>Edit</Button>
		<Button variant="danger" onclick={handleDelete}><Trash2 size={16} strokeWidth={2} aria-hidden="true" />Delete</Button>
	</PageHeader>

	<div class="p-6">
		{#if error}
			<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}

		<!-- Camera Details -->
		<FadeIn delay={0}>
		<div class="mb-8 grid grid-cols-2 gap-x-8 gap-y-3 rounded-lg border border-border bg-surface-raised p-5">
			<div>
				<span class="text-xs text-text-muted">Format</span>
				<p class="text-sm">{camera.format}</p>
			</div>
			<div>
				<span class="text-xs text-text-muted">Lens Mount</span>
				<p class="text-sm">{mountNameById[camera.lens_mount_id] ?? '—'}</p>
			</div>
			{#if defaultLensDisplay}
				<div>
					<span class="text-xs text-text-muted">Default Lens</span>
					<p class="text-sm">{defaultLensDisplay}</p>
				</div>
			{/if}
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
		</FadeIn>

		<!-- Maintenance History -->
		<FadeIn delay={50}>
		<div class="mb-8">
			<div class="mb-3 flex items-center justify-between">
				<h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">Maintenance History</h2>
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
		</FadeIn>

		<!-- Lenses -->
		<FadeIn delay={100}>
		{#if isFixedLens}
			<div class="mb-8">
				<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Built-in Lens</h2>
				{#if linkedLenses.length > 0}
					{@const lens = linkedLenses[0]}
					<div class="rounded-lg border border-border bg-surface-raised p-3">
						<div class="flex items-center gap-2">
							<span class="text-sm">{lensDisplayName(lens)}</span>
							{#if lens.max_aperture}
								<span class="text-xs text-text-faint">f/{lens.max_aperture}</span>
							{/if}
						</div>
					</div>
				{:else}
					<p class="text-sm text-text-faint">No lens data recorded.</p>
				{/if}
			</div>
		{:else}
			<div class="mb-8">
				<div class="mb-3 flex items-center justify-between">
					<h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">Compatible Lenses</h2>
					<Button size="sm" onclick={() => { linkLensId = ''; showLinkLensDialog = true; }}>+ Link Lens</Button>
				</div>
				{#if linkedLenses.length === 0}
					<p class="text-sm text-text-faint">No lenses linked yet. Link your {mountNameById[camera.lens_mount_id] ?? ''} mount lenses to set a default for new rolls.</p>
				{:else}
					<div class="mb-3">
						<Select
							label="Default Lens"
							bind:value={defaultLensId}
							options={defaultLensOptions}
							onchange={handleDefaultLensChange}
						/>
					</div>
					<div class="space-y-2">
						{#each linkedLenses as lens}
							<div class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised p-3">
								<div class="flex items-center gap-2">
									<span class="text-sm">{lensDisplayName(lens)}</span>
									{#if mountNameById[lens.lens_mount_id]}
										<span class="text-xs text-text-faint">{mountNameById[lens.lens_mount_id]}</span>
									{/if}
									{#if lens.date_sold}
										<span class="rounded bg-red-500/15 px-1.5 py-0.5 text-xs text-red-400">Sold</span>
									{/if}
								</div>
								<Button size="sm" variant="ghost" class="opacity-0 group-hover:opacity-100 transition-opacity focus:opacity-100 pointer-coarse:opacity-100" onclick={() => handleUnlinkLens(lens.id)}>&times;</Button>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
		</FadeIn>

		<!-- Rolls shot with this camera -->
		<FadeIn delay={150}>
		<div>
			<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
				Rolls ({rolls.length})
				<div class="flex-1 border-b border-border-subtle"></div>
			</h2>
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
		</FadeIn>
	</div>
{/if}

<!-- Add Maintenance Dialog -->
<Dialog bind:open={showMaintenanceDialog} title="Add Maintenance Record">
	<div class="space-y-4">
		<Select label="Type" bind:value={maintType} options={maintTypeOptions} />
		<ComboInput label="Done By" bind:value={maintDoneBy} placeholder="Garry's Camera Repair" options={maintProviderOptions} />
		<div class="grid grid-cols-2 gap-4">
			<DateInput label="Date" bind:value={maintDateDone} />
			<Input label="Cost ($)" bind:value={maintCost} type="number" step="0.01" placeholder="0.00" />
		</div>
		<Textarea label="Notes" bind:value={maintNotes} placeholder="What was done..." />
		<div class="flex justify-end gap-2 pt-2">
			<Button variant="ghost" onclick={() => (showMaintenanceDialog = false)}>Cancel</Button>
			<Button variant="primary" disabled={!!maintDateError} onclick={addMaintenance}>Add Record</Button>
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

<!-- Link Lens Dialog -->
<Dialog bind:open={showLinkLensDialog} title="Link Lens to Camera">
	<div class="space-y-4">
		{#if unlinkedLenses.length === 0}
			<p class="text-sm text-text-muted">All your lenses are already linked to this camera.</p>
		{:else}
			<Select label="Lens" bind:value={linkLensId} options={linkLensOptions} />
		{/if}
		<div class="flex justify-end gap-2 pt-2">
			<Button variant="ghost" onclick={() => (showLinkLensDialog = false)}>Cancel</Button>
			{#if unlinkedLenses.length > 0}
				<Button variant="primary" onclick={handleLinkLens} disabled={!linkLensId}>Link Lens</Button>
			{/if}
		</div>
	</div>
</Dialog>

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
