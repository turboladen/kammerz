<script lang="ts">
	import { Search } from 'lucide-svelte';

	interface SelectOption {
		value: string;
		label: string;
	}

	interface Props {
		searchQuery: string;
		groupBy: string;
		sortBy: string;
		groupByOptions: SelectOption[];
		sortOptions: SelectOption[];
		resultCount: number;
		totalCount: number;
		placeholder?: string;
	}

	let {
		searchQuery = $bindable(),
		groupBy = $bindable(),
		sortBy = $bindable(),
		groupByOptions,
		sortOptions,
		resultCount,
		totalCount,
		placeholder = 'Search...'
	}: Props = $props();

	const countLabel = $derived(
		resultCount < totalCount
			? `${resultCount} of ${totalCount}`
			: `${resultCount}`
	);
</script>

<div class="mb-3 flex flex-wrap items-center gap-3">
	<!-- Search input (full row on narrow screens, flexes inline on sm+) -->
	<div class="relative basis-full sm:basis-0 sm:flex-1">
		<div class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-text-faint">
			<Search size={14} strokeWidth={1.75} />
		</div>
		<input
			type="text"
			bind:value={searchQuery}
			{placeholder}
			aria-label="Search"
			spellcheck="false"
			class="h-[38px] w-full rounded-lg border border-border bg-surface pl-8 pr-3 text-sm
				text-text placeholder-text-faint transition-colors
				focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
		/>
	</div>

	<!-- Group by -->
	<label class="flex items-center gap-1.5">
		<span class="text-xs text-text-faint">Group</span>
		<select
			bind:value={groupBy}
			class="h-[38px] min-w-[120px] rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text
				transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
		>
			{#each groupByOptions as opt}
				<option value={opt.value}>{opt.label}</option>
			{/each}
		</select>
	</label>

	<!-- Sort by -->
	<label class="flex items-center gap-1.5">
		<span class="text-xs text-text-faint">Sort</span>
		<select
			bind:value={sortBy}
			class="h-[38px] min-w-[120px] rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text
				transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
		>
			{#each sortOptions as opt}
				<option value={opt.value}>{opt.label}</option>
			{/each}
		</select>
	</label>

	<!-- Result count -->
	<span class="shrink-0 font-mono text-xs text-text-faint">{countLabel}</span>
</div>
