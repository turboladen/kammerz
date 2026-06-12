<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import ListToolbar from '$lib/components/ui/ListToolbar.svelte';
	import GroupHeader from '$lib/components/ui/GroupHeader.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import { TestTube2 } from 'lucide-svelte';
	import { listAllSelfDevelopments } from '$lib/api/development';
	import { filterBySearch, groupItems, sortByString, sortByDate } from '$lib/utils/list';
	import { secondsToMmSs } from '$lib/utils/duration';
	import type { SelfDevListItem } from '$lib/types';

	let items: SelfDevListItem[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	// Toolbar state
	let searchQuery = $state('');
	let groupBy = $state('developer');
	let sortBy = $state('date-desc');

	// Pipeline: search → sort → group
	const afterSearch = $derived(
		filterBySearch(items, searchQuery, (d) =>
			[
				d.developer ?? '',
				d.developer_dilution ?? '',
				d.fixer ?? '',
				d.fixer_dilution ?? '',
				d.stop_bath ?? '',
				d.temperature ?? '',
				d.agitation_notes ?? '',
				d.film_stock_brand ?? '',
				d.film_stock_name ?? '',
				d.camera_brand ?? '',
				d.camera_model ?? '',
				d.roll_id,
				d.notes ?? ''
			].join(' ')
		)
	);

	const afterSort = $derived.by(() => {
		switch (sortBy) {
			case 'date-asc':
				return sortByDate(afterSearch, (d) => d.dev_date, 'asc');
			case 'date-added-desc':
				return sortByDate(afterSearch, (d) => d.created_at, 'desc');
			case 'developer-asc':
				return sortByString(afterSearch, (d) => d.developer, 'asc');
			default:
				return sortByDate(afterSearch, (d) => d.dev_date, 'desc');
		}
	});

	const grouped = $derived.by(() => {
		if (groupBy === 'developer') return groupItems(afterSort, (d) => d.developer ?? 'Unknown Developer');
		if (groupBy === 'film-stock')
			return groupItems(afterSort, (d) =>
				d.film_stock_brand ? `${d.film_stock_brand} ${d.film_stock_name}` : 'No Film Stock'
			);
		return groupItems(afterSort, () => '');
	});

	const resultCount = $derived(afterSearch.length);

	const groupByOptions = [
		{ value: 'developer', label: 'Developer' },
		{ value: 'film-stock', label: 'Film Stock' },
		{ value: 'none', label: 'None' }
	];

	const sortOptions = [
		{ value: 'date-desc', label: 'Newest Processed' },
		{ value: 'date-asc', label: 'Oldest Processed' },
		{ value: 'date-added-desc', label: 'Recently Added' },
		{ value: 'developer-asc', label: 'Developer A\u2013Z' }
	];

	function filmStockLabel(d: SelfDevListItem): string {
		if (!d.film_stock_brand) return '';
		let label = `${d.film_stock_brand} ${d.film_stock_name}`;
		if (d.film_stock_iso) label += ` (${d.film_stock_iso})`;
		return label;
	}

	async function load() {
		try {
			items = await listAllSelfDevelopments();
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

<PageHeader title="Developments" description="Self-development records across all rolls" />

<div class="p-6">
	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	<ListToolbar
		bind:searchQuery
		bind:groupBy
		bind:sortBy
		{groupByOptions}
		{sortOptions}
		{resultCount}
		totalCount={items.length}
		placeholder="Search by developer, film stock, chemistry..."
	/>

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if resultCount === 0 && items.length === 0}
		<EmptyState
			title="No Developments"
			message="Self-development records will appear here once you add them to your rolls."
		>
			{#snippet icon()}<TestTube2 size={24} strokeWidth={1.5} />{/snippet}
		</EmptyState>
	{:else if resultCount === 0}
		<p class="mt-6 text-center text-sm text-text-muted">No developments match your search.</p>
	{:else}
		{#each Object.entries(grouped) as [groupKey, groupItems]}
			<GroupHeader label={groupKey} />
			<div class="mb-4 grid gap-2">
				{#each groupItems as d, i}
					<FadeIn delay={Math.min(i, 10) * 30}>
						<a
							href="/rolls/{d.roll_pk}?from=developments"
							class="group rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px block"
						>
							<!-- Top row: chemistry + roll context -->
							<div class="flex items-start justify-between gap-4">
								<div class="min-w-0 flex-1">
									<div class="flex items-center gap-2">
										{#if d.developer}
											<span class="text-sm font-semibold text-text">
												{d.developer}{#if d.developer_dilution}<span class="font-normal text-text-muted">
														({d.developer_dilution})</span
													>{/if}
											</span>
										{:else}
											<span class="text-sm italic text-text-faint">No developer recorded</span>
										{/if}
										{#if d.temperature}
											<span class="text-xs text-text-faint">{d.temperature}</span>
										{/if}
									</div>
									{#if d.fixer || d.stop_bath}
										<div class="mt-0.5 flex flex-wrap gap-x-3 text-xs text-text-muted">
											{#if d.fixer}
												<span
													>Fixer: {d.fixer}{#if d.fixer_dilution}
														({d.fixer_dilution}){/if}</span
												>
											{/if}
											{#if d.stop_bath}
												<span>Stop: {d.stop_bath}</span>
											{/if}
										</div>
									{/if}
								</div>
								<div class="flex shrink-0 items-center gap-3 text-right">
									{#if filmStockLabel(d)}
										<span class="text-xs text-text-muted">{filmStockLabel(d)}</span>
									{/if}
									<span class="font-mono text-xs text-text-faint">{d.roll_id}</span>
									<Badge status={d.roll_status} />
									{#if d.date_processed}
										<span class="text-xs text-text-faint">{d.date_processed}</span>
									{/if}
									<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100"
										>&rarr;</span
									>
								</div>
							</div>

							<!-- Stages -->
							{#if d.stages.length > 0}
								<div class="mt-2 flex flex-wrap gap-x-4 gap-y-0.5">
									{#each d.stages as stage, si}
										<span class="text-xs text-text-faint">
											<span class="font-mono text-text-muted">{si + 1}.</span>
											{stage.stage_name}{#if stage.duration_seconds}
												<span class="font-mono"> {secondsToMmSs(stage.duration_seconds)}</span>
											{/if}
										</span>
									{/each}
								</div>
							{/if}

							<!-- Notes (truncated) -->
							{#if d.agitation_notes || d.notes}
								<div class="mt-1 truncate text-xs italic text-text-faint">
									{d.agitation_notes ?? d.notes}
								</div>
							{/if}
						</a>
					</FadeIn>
				{/each}
			</div>
		{/each}
	{/if}
</div>
