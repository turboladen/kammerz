<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import FilmLeader from '$lib/components/ui/FilmLeader.svelte';
	import ListToolbar from '$lib/components/ui/ListToolbar.svelte';
	import GroupHeader from '$lib/components/ui/GroupHeader.svelte';
	import RollRow from '$lib/components/rolls/RollRow.svelte';
	import { Film } from 'lucide-svelte';
	import { listRolls } from '$lib/api/rolls';
	import { filterBySearch, groupItems, sortByString, sortByDate } from '$lib/utils/list';
	import { statusConfig } from '$lib/utils/status';
	import type { RollWithDetails, RollStatus } from '$lib/types';

	let rolls: RollWithDetails[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let filterStatus = $state(page.url.searchParams.get('status') ?? 'all');

	// Toolbar state — seeded from the URL so Back from a roll restores the view (kammerz-pq8)
	let searchQuery = $state(page.url.searchParams.get('q') ?? '');
	let groupBy = $state(page.url.searchParams.get('group') ?? 'none');
	let sortBy = $state(page.url.searchParams.get('sort') ?? 'date-loaded-desc');

	// Reflect toolbar state in the URL (kammerz-pq8). Reads only the state vars, so
	// goto() — which changes the URL but not these vars — can't cause an effect loop.
	$effect(() => {
		const params = new URLSearchParams();
		if (searchQuery) params.set('q', searchQuery);
		if (filterStatus !== 'all') params.set('status', filterStatus);
		if (groupBy !== 'none') params.set('group', groupBy);
		if (sortBy !== 'date-loaded-desc') params.set('sort', sortBy);
		const qs = params.toString();
		goto(qs ? `?${qs}` : '/rolls', { replaceState: true, keepFocus: true, noScroll: true });
	});

	const statuses: { value: string; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'loaded', label: 'Loaded' },
		{ value: 'shooting', label: 'Shooting' },
		{ value: 'shot', label: 'Shot' },
		{ value: 'at-lab', label: 'At Lab' },
		{ value: 'lab-done', label: 'Lab Done' },
		{ value: 'developing', label: 'Developing' },
		{ value: 'developed', label: 'Developed' },
		{ value: 'scanned', label: 'Scanned' },
		{ value: 'post-processed', label: 'Post-processed' },
		{ value: 'archived', label: 'Archived' }
	];

	// Pipeline: status filter → search → sort → group
	const afterStatusFilter = $derived(filterStatus === 'all' ? rolls : rolls.filter((r) => r.status === filterStatus));

	const afterSearch = $derived(
		filterBySearch(afterStatusFilter, searchQuery, (r) =>
			[
				r.roll_id,
				r.camera_brand ?? '',
				r.camera_model ?? '',
				r.film_stock_brand ?? '',
				r.film_stock_name ?? '',
				r.lens_brand ?? '',
				r.lens_name ?? '',
				r.notes ?? ''
			].join(' ')
		)
	);

	const afterSort = $derived.by(() => {
		switch (sortBy) {
			case 'date-loaded-asc':
				return sortByDate(afterSearch, (r) => r.date_loaded, 'asc');
			case 'date-finished-desc':
				return sortByDate(afterSearch, (r) => r.date_finished, 'desc');
			case 'date-finished-asc':
				return sortByDate(afterSearch, (r) => r.date_finished, 'asc');
			case 'roll-id-asc':
				return sortByString(afterSearch, (r) => r.roll_id, 'asc');
			case 'roll-id-desc':
				return sortByString(afterSearch, (r) => r.roll_id, 'desc');
			case 'date-added-desc':
				return sortByDate(afterSearch, (r) => r.created_at, 'desc');
			default:
				return sortByDate(afterSearch, (r) => r.date_loaded, 'desc');
		}
	});

	const grouped = $derived.by(() => {
		if (groupBy === 'status') return groupItems(afterSort, (r) => statusConfig[r.status]?.label ?? r.status);
		if (groupBy === 'camera')
			return groupItems(afterSort, (r) => (r.camera_brand ? `${r.camera_brand} ${r.camera_model}` : 'No Camera'));
		if (groupBy === 'film-stock')
			return groupItems(afterSort, (r) =>
				r.film_stock_brand ? `${r.film_stock_brand} ${r.film_stock_name}` : 'No Film Stock'
			);
		return groupItems(afterSort, () => '');
	});

	const resultCount = $derived(afterSearch.length);

	const groupByOptions = [
		{ value: 'none', label: 'None' },
		{ value: 'status', label: 'Status' },
		{ value: 'camera', label: 'Camera' },
		{ value: 'film-stock', label: 'Film Stock' }
	];

	const sortOptions = [
		{ value: 'date-loaded-desc', label: 'Newest Loaded' },
		{ value: 'date-loaded-asc', label: 'Oldest Loaded' },
		{ value: 'date-finished-desc', label: 'Newest Finished' },
		{ value: 'date-finished-asc', label: 'Oldest Finished' },
		{ value: 'roll-id-asc', label: 'Roll ID A\u2013Z' },
		{ value: 'roll-id-desc', label: 'Roll ID Z\u2013A' },
		{ value: 'date-added-desc', label: 'Recently Added' }
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
		totalCount={afterStatusFilter.length}
		placeholder="Search rolls..."
	/>

	<!-- Status filter tabs -->
	<div class="mb-4 flex flex-wrap gap-2">
		{#each statuses as s}
			<Button
				size="sm"
				variant={filterStatus === s.value ? 'primary' : 'ghost'}
				onclick={() => (filterStatus = s.value)}>{s.label}</Button
			>
		{/each}
	</div>

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if resultCount === 0 && rolls.length === 0}
		<EmptyState title="No Rolls" message="Create your first roll to get started.">
			{#snippet art()}<FilmLeader />{/snippet}
			<Button variant="primary" href="/rolls/new">+ New Roll</Button>
		</EmptyState>
	{:else if resultCount === 0}
		{#if filterStatus !== 'all' && !searchQuery}
			<EmptyState
				title={`No ${statuses.find((s) => s.value === filterStatus)?.label ?? filterStatus} Rolls`}
				message="Try a different filter."
			>
				{#snippet icon()}<Film size={24} strokeWidth={1.5} />{/snippet}
			</EmptyState>
		{:else}
			<p class="mt-6 text-center text-sm text-text-muted">
				{searchQuery ? 'No rolls match your search.' : 'No rolls match the current filters.'}
			</p>
		{/if}
	{:else}
		{#each Object.entries(grouped) as [groupKey, groupRolls]}
			<GroupHeader label={groupKey} />
			<div class="mb-4 grid gap-1.5">
				{#each groupRolls as roll, i}
					<FadeIn delay={Math.min(i, 10) * 30}>
						<RollRow {roll} href="/rolls/{roll.id}">
							{#snippet trailing()}
								<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
							{/snippet}
						</RollRow>
					</FadeIn>
				{/each}
			</div>
		{/each}
	{/if}
</div>
