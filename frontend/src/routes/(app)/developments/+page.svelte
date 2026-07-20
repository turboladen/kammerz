<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import ListToolbar from '$lib/components/ui/ListToolbar.svelte';
	import GroupHeader from '$lib/components/ui/GroupHeader.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { TestTube2, FlaskConical } from 'lucide-svelte';
	import { listAllSelfDevelopments, listAllLabDevelopments } from '$lib/api/development';
	import { filterBySearch, groupItems, sortByString, sortByDate } from '$lib/utils/list';
	import { secondsToMmSs } from '$lib/utils/duration';
	import type { Snippet } from 'svelte';
	import type { SelfDevListItem, LabDevListItem } from '$lib/types';

	let tab = $state<'self' | 'lab'>('self');

	let selfItems: SelfDevListItem[] = $state([]);
	let labItems: LabDevListItem[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	// Toolbar state (shared across tabs; group/sort options swap with the tab)
	let searchQuery = $state('');
	let groupBy = $state('developer');
	let sortBy = $state('date-desc');

	// --- Self pipeline: search → sort → group ---
	const selfAfterSearch = $derived(
		filterBySearch(selfItems, searchQuery, (d) =>
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

	const selfAfterSort = $derived.by(() => {
		switch (sortBy) {
			case 'date-asc':
				return sortByDate(selfAfterSearch, (d) => d.dev_date, 'asc');
			case 'date-added-desc':
				return sortByDate(selfAfterSearch, (d) => d.created_at, 'desc');
			case 'developer-asc':
				return sortByString(selfAfterSearch, (d) => d.developer, 'asc');
			default:
				return sortByDate(selfAfterSearch, (d) => d.dev_date, 'desc');
		}
	});

	const selfGrouped = $derived.by(() => {
		if (groupBy === 'developer') return groupItems(selfAfterSort, (d) => d.developer ?? 'Unknown Developer');
		if (groupBy === 'film-stock')
			return groupItems(selfAfterSort, (d) =>
				d.film_stock_brand ? `${d.film_stock_brand} ${d.film_stock_name}` : 'No Film Stock'
			);
		return groupItems(selfAfterSort, () => '');
	});

	// --- Lab pipeline: search → sort → group ---
	const labAfterSearch = $derived(
		filterBySearch(labItems, searchQuery, (d) =>
			[
				d.lab_name ?? '',
				d.film_stock_brand ?? '',
				d.film_stock_name ?? '',
				d.camera_brand ?? '',
				d.camera_model ?? '',
				d.roll_id,
				d.notes ?? ''
			].join(' ')
		)
	);

	const labAfterSort = $derived.by(() => {
		switch (sortBy) {
			case 'date-asc':
				return sortByDate(labAfterSearch, (d) => d.dev_date, 'asc');
			case 'date-added-desc':
				return sortByDate(labAfterSearch, (d) => d.created_at, 'desc');
			case 'lab-asc':
				return sortByString(labAfterSearch, (d) => d.lab_name, 'asc');
			default:
				return sortByDate(labAfterSearch, (d) => d.dev_date, 'desc');
		}
	});

	const labGrouped = $derived.by(() => {
		if (groupBy === 'lab') return groupItems(labAfterSort, (d) => d.lab_name ?? 'Unknown Lab');
		if (groupBy === 'film-stock')
			return groupItems(labAfterSort, (d) =>
				d.film_stock_brand ? `${d.film_stock_brand} ${d.film_stock_name}` : 'No Film Stock'
			);
		return groupItems(labAfterSort, () => '');
	});

	// --- Active-tab views ---
	const totalCount = $derived(tab === 'self' ? selfItems.length : labItems.length);
	const resultCount = $derived(tab === 'self' ? selfAfterSearch.length : labAfterSearch.length);

	const groupByOptions = $derived(
		tab === 'self'
			? [
					{ value: 'developer', label: 'Developer' },
					{ value: 'film-stock', label: 'Film Stock' },
					{ value: 'none', label: 'None' }
				]
			: [
					{ value: 'lab', label: 'Lab' },
					{ value: 'film-stock', label: 'Film Stock' },
					{ value: 'none', label: 'None' }
				]
	);

	const sortOptions = $derived(
		tab === 'self'
			? [
					{ value: 'date-desc', label: 'Newest Processed' },
					{ value: 'date-asc', label: 'Oldest Processed' },
					{ value: 'date-added-desc', label: 'Recently Added' },
					{ value: 'developer-asc', label: 'Developer A–Z' }
				]
			: [
					{ value: 'date-desc', label: 'Newest Activity' },
					{ value: 'date-asc', label: 'Oldest Activity' },
					{ value: 'date-added-desc', label: 'Recently Added' },
					{ value: 'lab-asc', label: 'Lab A–Z' }
				]
	);

	// Reset the toolbar to the active tab's defaults so a stale group/sort value
	// from the other tab (e.g. 'developer' on the lab tab) doesn't leak through.
	function switchTab(next: 'self' | 'lab') {
		if (next === tab) return;
		tab = next;
		searchQuery = '';
		groupBy = next === 'self' ? 'developer' : 'lab';
		sortBy = 'date-desc';
	}

	function filmStockLabel(d: SelfDevListItem | LabDevListItem): string {
		if (!d.film_stock_brand) return '';
		let label = `${d.film_stock_brand} ${d.film_stock_name}`;
		if (d.film_stock_iso) label += ` (${d.film_stock_iso})`;
		return label;
	}

	async function load() {
		try {
			[selfItems, labItems] = await Promise.all([listAllSelfDevelopments(), listAllLabDevelopments()]);
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

<PageHeader title="Developments" description="Development records across all rolls" />

<div class="p-6">
	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	<div class="mb-4 flex gap-2">
		<Button variant={tab === 'self' ? 'primary' : 'ghost'} size="sm" onclick={() => switchTab('self')}>Self</Button>
		<Button variant={tab === 'lab' ? 'primary' : 'ghost'} size="sm" onclick={() => switchTab('lab')}>Lab</Button>
	</div>

	<ListToolbar
		bind:searchQuery
		bind:groupBy
		bind:sortBy
		{groupByOptions}
		{sortOptions}
		{resultCount}
		{totalCount}
		placeholder={tab === 'self'
			? 'Search by developer, film stock, chemistry...'
			: 'Search by lab, film stock, roll...'}
	/>

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if resultCount === 0 && totalCount === 0}
		{#if tab === 'self'}
			<EmptyState
				title="No Self-Developments"
				message="Self-development records will appear here once you add them to your rolls."
			>
				{#snippet icon()}<TestTube2 size={24} strokeWidth={1.5} />{/snippet}
			</EmptyState>
		{:else}
			<EmptyState
				title="No Lab Developments"
				message="Lab development records will appear here once you send rolls to a lab."
			>
				{#snippet icon()}<FlaskConical size={24} strokeWidth={1.5} />{/snippet}
			</EmptyState>
		{/if}
	{:else if resultCount === 0}
		<p class="mt-6 text-center text-sm text-text-muted">No developments match your search.</p>
	{:else if tab === 'self'}
		{#each Object.entries(selfGrouped) as [groupKey, groupRows]}
			<GroupHeader label={groupKey} />
			<div class="mb-4 grid gap-2">
				{#each groupRows as d, i}
					{@render row(d, i, selfBody)}
				{/each}
			</div>
		{/each}
	{:else}
		{#each Object.entries(labGrouped) as [groupKey, groupRows]}
			<GroupHeader label={groupKey} />
			<div class="mb-4 grid gap-2">
				{#each groupRows as d, i}
					{@render row(d, i, labBody)}
				{/each}
			</div>
		{/each}
	{/if}
</div>

<!-- Shared row scaffold: FadeIn + anchor + right-cluster + notes are identical
     across both tabs (keeping their styling in lockstep); only the left column
     and the self-only stages strip diverge, supplied via the `body` snippet. -->
{#snippet row(d: SelfDevListItem | LabDevListItem, i: number, body: Snippet<[SelfDevListItem | LabDevListItem]>)}
	<FadeIn delay={Math.min(i, 10) * 30}>
		<a
			href="/rolls/{d.roll_pk}?from=developments"
			class="group rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px block"
		>
			{@render body(d)}
		</a>
	</FadeIn>
{/snippet}

<!-- Right-hand context cluster shared by both row bodies. -->
{#snippet rightCluster(d: SelfDevListItem | LabDevListItem, trailingDate: string | null)}
	<div class="flex shrink-0 items-center gap-3 text-right">
		{#if filmStockLabel(d)}
			<span class="text-xs text-text-muted">{filmStockLabel(d)}</span>
		{/if}
		<span class="font-mono text-xs text-text-faint">{d.roll_id}</span>
		<Badge badge={d.badge} groupKey={d.group_key} />
		{#if trailingDate}
			<span class="text-xs text-text-faint">{trailingDate}</span>
		{/if}
		<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
	</div>
{/snippet}

{#snippet selfBody(d: SelfDevListItem | LabDevListItem)}
	{@const s = d as SelfDevListItem}
	<!-- Top row: chemistry + roll context -->
	<div class="flex items-start justify-between gap-4">
		<div class="min-w-0 flex-1">
			<div class="flex items-center gap-2">
				{#if s.developer}
					<span class="text-sm font-semibold text-text">
						{s.developer}{#if s.developer_dilution}<span class="font-normal text-text-muted">
								({s.developer_dilution})</span
							>{/if}
					</span>
				{:else}
					<span class="text-sm italic text-text-faint">No developer recorded</span>
				{/if}
				{#if s.temperature}
					<span class="text-xs text-text-faint">{s.temperature}</span>
				{/if}
			</div>
			{#if s.fixer || s.stop_bath}
				<div class="mt-0.5 flex flex-wrap gap-x-3 text-xs text-text-muted">
					{#if s.fixer}
						<span
							>Fixer: {s.fixer}{#if s.fixer_dilution}
								({s.fixer_dilution}){/if}</span
						>
					{/if}
					{#if s.stop_bath}
						<span>Stop: {s.stop_bath}</span>
					{/if}
				</div>
			{/if}
		</div>
		{@render rightCluster(s, s.date_processed)}
	</div>

	<!-- Stages -->
	{#if s.stages.length > 0}
		<div class="mt-2 flex flex-wrap gap-x-4 gap-y-0.5">
			{#each s.stages as stage, si}
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
	{#if s.agitation_notes || s.notes}
		<div class="mt-1 truncate text-xs italic text-text-faint">
			{s.agitation_notes ?? s.notes}
		</div>
	{/if}
{/snippet}

{#snippet labBody(d: SelfDevListItem | LabDevListItem)}
	{@const l = d as LabDevListItem}
	<!-- Top row: lab + roll context -->
	<div class="flex items-start justify-between gap-4">
		<div class="min-w-0 flex-1">
			<div class="flex items-center gap-2">
				{#if l.lab_name}
					<span class="text-sm font-semibold text-text">{l.lab_name}</span>
				{:else}
					<span class="text-sm italic text-text-faint">No lab recorded</span>
				{/if}
				{#if l.cost != null}
					<span class="font-mono text-xs text-text-faint">${l.cost.toFixed(2)}</span>
				{/if}
			</div>
			<div class="mt-0.5 flex flex-wrap gap-x-3 text-xs text-text-muted">
				{#if l.date_dropped_off}
					<span>Dropped off: {l.date_dropped_off}</span>
				{/if}
				{#if l.date_received}
					<span>Received: {l.date_received}</span>
				{/if}
			</div>
		</div>
		{@render rightCluster(l, null)}
	</div>

	<!-- Notes (truncated) -->
	{#if l.notes}
		<div class="mt-1 truncate text-xs italic text-text-faint">
			{l.notes}
		</div>
	{/if}
{/snippet}
