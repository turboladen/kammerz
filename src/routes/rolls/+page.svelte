<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import { Film } from 'lucide-svelte';
	import { listRolls } from '$lib/api/rolls';
	import type { RollWithDetails, RollStatus } from '$lib/types';

	let rolls: RollWithDetails[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let filterStatus = $state('all');

	const filtered = $derived(
		filterStatus === 'all' ? rolls : rolls.filter((r) => r.status === filterStatus)
	);

	const statuses: { value: string; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'loaded', label: 'Loaded' },
		{ value: 'shooting', label: 'Shooting' },
		{ value: 'shot', label: 'Shot' },
		{ value: 'at-lab', label: 'At Lab' },
		{ value: 'developing', label: 'Developing' },
		{ value: 'developed', label: 'Developed' },
		{ value: 'scanned', label: 'Scanned' },
		{ value: 'archived', label: 'Archived' }
	];

	async function load() {
		try {
			rolls = await listRolls();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="Rolls" description="All film rolls">
	<div class="flex items-center gap-2">
		<Button variant="ghost" href="/import">Import Notes</Button>
		<Button variant="primary" href="/rolls/new">+ New Roll</Button>
	</div>
</PageHeader>

<div class="p-6">
	<!-- Status filter tabs -->
	<div class="mb-4 flex flex-wrap gap-2">
		{#each statuses as s}
			<Button
				size="sm"
				variant={filterStatus === s.value ? 'primary' : 'ghost'}
				onclick={() => (filterStatus = s.value)}
			>{s.label}</Button>
		{/each}
	</div>

	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if filtered.length === 0}
		<EmptyState
			title={filterStatus === 'all' ? 'No Rolls' : `No "${filterStatus}" Rolls`}
			message={filterStatus === 'all' ? 'Create your first roll to get started.' : 'Try a different filter.'}
		>
			{#snippet icon()}<Film size={24} strokeWidth={1.5} />{/snippet}
			{#if filterStatus === 'all'}
				<Button variant="primary" href="/rolls/new">+ New Roll</Button>
			{/if}
		</EmptyState>
	{:else}
		<div class="grid gap-2">
			{#each filtered as roll, i}
				<FadeIn delay={Math.min(i, 10) * 30}>
					<a
						href="/rolls/{roll.id}"
						class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised p-4 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
					>
						<div class="flex items-center gap-4">
							<span class="font-mono text-sm font-semibold">{roll.roll_id}</span>
							<Badge status={roll.status} />
							{#if roll.camera_brand}
								<span class="text-sm text-text-muted">{roll.camera_brand} {roll.camera_model}</span>
							{:else}
								<span class="text-sm italic text-text-faint">No camera</span>
							{/if}
						</div>
						<div class="flex items-center gap-4">
							{#if roll.film_stock_brand}
								<span class="text-xs text-text-muted">{roll.film_stock_brand} {roll.film_stock_name}</span>
							{/if}
							{#if roll.lens_brand}
								<span class="text-xs text-text-faint">{roll.lens_brand} {roll.lens_name}</span>
							{/if}
							{#if roll.date_loaded}
								<span class="text-xs text-text-faint">{roll.date_loaded}</span>
							{/if}
							<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
						</div>
					</a>
				</FadeIn>
			{/each}
		</div>
	{/if}
</div>
