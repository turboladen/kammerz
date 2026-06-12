<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import { FlaskConical } from 'lucide-svelte';
	import { listLabs, createLab, updateLab, deleteLab } from '$lib/api/labs';
	import type { Lab, LabInsert } from '$lib/types';

	let labs: Lab[] = $state([]);
	let loading = $state(true);
	let showAddDialog = $state(false);
	let editingLab: Lab | null = $state(null);
	let deletingLab: Lab | null = $state(null);
	let error = $state('');

	let name = $state('');
	let location = $state('');
	let website = $state('');
	let notes = $state('');

	async function load() {
		try {
			labs = await listLabs();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	function resetForm() {
		name = '';
		location = '';
		website = '';
		notes = '';
	}

	function openAddDialog() {
		resetForm();
		error = '';
		showAddDialog = true;
	}

	async function handleAdd() {
		error = '';
		if (!name.trim()) {
			error = 'Lab name is required.';
			return;
		}
		try {
			const lab: LabInsert = {
				name: name.trim(),
				location: location || null,
				website: website || null,
				notes: notes || null
			};
			await createLab(lab);
			showAddDialog = false;
			resetForm();
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	function startEdit(lab: Lab) {
		error = '';
		editingLab = lab;
		name = lab.name;
		location = lab.location ?? '';
		website = lab.website ?? '';
		notes = lab.notes ?? '';
	}

	async function handleEdit() {
		if (!editingLab) return;
		error = '';
		if (!name.trim()) {
			error = 'Lab name is required.';
			return;
		}
		try {
			await updateLab(editingLab.id, {
				name: name.trim(),
				location: location || null,
				website: website || null,
				notes: notes || null
			});
			editingLab = null;
			resetForm();
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	function handleDelete(lab: Lab) {
		deletingLab = lab;
	}

	async function confirmDelete() {
		if (!deletingLab) return;
		const lab = deletingLab;
		// Close the dialog before the request — a failure is reported via the
		// page error banner, and the dialog stays re-openable.
		deletingLab = null;
		error = '';
		try {
			await deleteLab(lab.id);
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="Labs" description="Development labs you use">
	<Button variant="primary" onclick={openAddDialog}>+ Add Lab</Button>
</PageHeader>

<div class="p-6">
	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if labs.length === 0}
		<EmptyState title="No Labs" message="Add the labs you use for film development.">
			{#snippet icon()}<FlaskConical size={24} strokeWidth={1.5} />{/snippet}
			<Button variant="primary" onclick={openAddDialog}>+ Add Lab</Button>
		</EmptyState>
	{:else}
		<div class="grid gap-3">
			{#each labs as lab}
				<div
					class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised p-4 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
				>
					<div>
						<span class="font-semibold">{lab.name}</span>
						<div class="mt-1 flex gap-3 text-xs text-text-muted">
							{#if lab.location}
								<span>{lab.location}</span>
							{/if}
							{#if lab.website}
								<span>{lab.website}</span>
							{/if}
						</div>
						{#if lab.notes}
							<p class="mt-1 text-sm text-text-muted">{lab.notes}</p>
						{/if}
					</div>
					<div
						class="flex gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100 pointer-coarse:opacity-100"
					>
						<Button size="sm" variant="ghost" onclick={() => startEdit(lab)}>Edit</Button>
						<Button size="sm" variant="ghost" onclick={() => handleDelete(lab)}>&times;</Button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<Dialog bind:open={showAddDialog} title="Add Lab" onclose={resetForm}>
	<div class="space-y-4">
		<Input label="Lab Name" bind:value={name} placeholder="The Darkroom" />
		<Input label="Location" bind:value={location} placeholder="San Clemente, CA" />
		<Input label="Website" bind:value={website} placeholder="thedarkroom.com" />
		<Textarea label="Notes" bind:value={notes} />
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
			<Button variant="primary" onclick={handleAdd}>Add Lab</Button>
		</div>
	</div>
</Dialog>

{#if editingLab}
	<Dialog
		open={true}
		title="Edit Lab"
		onclose={() => {
			editingLab = null;
			resetForm();
		}}
	>
		<div class="space-y-4">
			<Input label="Lab Name" bind:value={name} />
			<Input label="Location" bind:value={location} />
			<Input label="Website" bind:value={website} />
			<Textarea label="Notes" bind:value={notes} />
			{#if error}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
			{/if}
			<div class="flex justify-end gap-2 pt-2">
				<Button
					variant="ghost"
					onclick={() => {
						editingLab = null;
						resetForm();
					}}>Cancel</Button
				>
				<Button variant="primary" onclick={handleEdit}>Save</Button>
			</div>
		</div>
	</Dialog>
{/if}

{#if deletingLab}
	<ConfirmDialog
		open={true}
		title="Delete Lab"
		message={`Permanently delete ${deletingLab.name}?`}
		confirmLabel="Delete Lab"
		onconfirm={confirmDelete}
		oncancel={() => {
			deletingLab = null;
		}}
	/>
{/if}
