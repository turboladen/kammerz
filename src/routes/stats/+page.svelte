<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import { getCatalogStats } from '$lib/api/stats';
	import type { CatalogStats } from '$lib/types';

	let stats: CatalogStats | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	// Derived max values for bar scaling
	const maxMonthCount = $derived(
		stats ? Math.max(...stats.rolls_per_month.map((m) => m.count), 1) : 1
	);

	function formatCurrency(amount: number): string {
		return amount.toLocaleString('en-US', { style: 'currency', currency: 'USD' });
	}

	function formatMonth(ym: string): string {
		const [year, month] = ym.split('-');
		const date = new Date(parseInt(year), parseInt(month) - 1);
		return date.toLocaleDateString('en-US', { month: 'short', year: '2-digit' });
	}

	async function load() {
		try {
			stats = await getCatalogStats();
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

<PageHeader title="Statistics" description="Shooting activity and cost tracking" />

<div class="flex-1 p-6">
	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	{#if loading}
		<p class="text-sm text-text-muted">Loading statistics...</p>
	{:else if stats}
		<!-- Summary Cards -->
		<div class="mb-8 grid grid-cols-4 gap-4">
			<div class="rounded-lg border border-border bg-surface-raised p-4">
				<p class="font-mono text-2xl font-semibold">{stats.total_rolls}</p>
				<p class="text-xs text-text-faint">Total Rolls</p>
			</div>
			<div class="rounded-lg border border-border bg-surface-raised p-4">
				<p class="font-mono text-2xl font-semibold">{stats.total_shots}</p>
				<p class="text-xs text-text-faint">Total Shots</p>
			</div>
			<div class="rounded-lg border border-border bg-surface-raised p-4">
				<p class="font-mono text-2xl font-semibold">{formatCurrency(stats.total_cost)}</p>
				<p class="text-xs text-text-faint">Total Costs</p>
			</div>
			<div class="rounded-lg border border-border bg-surface-raised p-4">
				<p class="font-mono text-2xl font-semibold">{stats.total_cameras}</p>
				<p class="text-xs text-text-faint">Cameras</p>
			</div>
		</div>

		<!-- Cost Breakdown -->
		{#if stats.total_cost > 0}
			<div class="mb-8">
				<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Cost Breakdown</h2>
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<div class="mb-3 flex h-3 overflow-hidden rounded-full bg-surface-overlay">
						{#if stats.total_lab_dev_cost > 0}
							<div
								style="width: {(stats.total_lab_dev_cost / stats.total_cost) * 100}%"
								class="bg-accent transition-all duration-300"
								title="Lab Development: {formatCurrency(stats.total_lab_dev_cost)}"
							></div>
						{/if}
						{#if stats.total_maintenance_cost > 0}
							<div
								style="width: {(stats.total_maintenance_cost / stats.total_cost) * 100}%"
								class="bg-text-muted transition-all duration-300"
								title="Maintenance: {formatCurrency(stats.total_maintenance_cost)}"
							></div>
						{/if}
					</div>
					<div class="flex gap-6 text-xs">
						<span class="flex items-center gap-1.5 text-text-muted">
							<span class="h-2 w-2 rounded-full bg-accent"></span>
							Lab Development
							<span class="font-mono text-text-faint">{formatCurrency(stats.total_lab_dev_cost)}</span>
						</span>
						<span class="flex items-center gap-1.5 text-text-muted">
							<span class="h-2 w-2 rounded-full bg-text-muted"></span>
							Maintenance
							<span class="font-mono text-text-faint">{formatCurrency(stats.total_maintenance_cost)}</span>
						</span>
					</div>
				</div>
			</div>
		{/if}

		<!-- Rolls Per Month -->
		{#if stats.rolls_per_month.length > 0}
			<div class="mb-8">
				<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Rolls Per Month</h2>
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<div class="space-y-2">
						{#each stats.rolls_per_month as month}
							<div class="flex items-center gap-3">
								<span class="w-16 text-right text-xs text-text-muted">{formatMonth(month.month)}</span>
								<div class="flex-1">
									<div
										class="h-5 rounded bg-accent/80 transition-all duration-300"
										style="width: {(month.count / maxMonthCount) * 100}%"
									></div>
								</div>
								<span class="w-8 font-mono text-xs text-text-faint">{month.count}</span>
							</div>
						{/each}
					</div>
				</div>
			</div>
		{/if}

		<!-- Rankings Row -->
		<div class="mb-8 grid gap-6 sm:grid-cols-3">
			<!-- Top Film Stocks -->
			{#if stats.top_film_stocks.length > 0}
				<div>
					<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Top Film Stocks</h2>
					<div class="rounded-lg border border-border bg-surface-raised p-4">
						<div class="space-y-2">
							{#each stats.top_film_stocks as item, i}
								<div class="flex items-center justify-between">
									<span class="text-sm text-text-muted">
										<span class="font-mono text-xs text-text-faint">{i + 1}.</span>
										{item.label}
									</span>
									<span class="rounded-full bg-accent/15 px-2 py-0.5 font-mono text-xs text-accent">{item.count}</span>
								</div>
							{/each}
						</div>
					</div>
				</div>
			{/if}

			<!-- Top Cameras -->
			{#if stats.top_cameras.length > 0}
				<div>
					<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Top Cameras</h2>
					<div class="rounded-lg border border-border bg-surface-raised p-4">
						<div class="space-y-2">
							{#each stats.top_cameras as item, i}
								<div class="flex items-center justify-between">
									<span class="text-sm text-text-muted">
										<span class="font-mono text-xs text-text-faint">{i + 1}.</span>
										{item.label}
									</span>
									<span class="rounded-full bg-accent/15 px-2 py-0.5 font-mono text-xs text-accent">{item.count}</span>
								</div>
							{/each}
						</div>
					</div>
				</div>
			{/if}

			<!-- Top Lenses -->
			{#if stats.top_lenses.length > 0}
				<div>
					<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Top Lenses</h2>
					<div class="rounded-lg border border-border bg-surface-raised p-4">
						<div class="space-y-2">
							{#each stats.top_lenses as item, i}
								<div class="flex items-center justify-between">
									<span class="text-sm text-text-muted">
										<span class="font-mono text-xs text-text-faint">{i + 1}.</span>
										{item.label}
									</span>
									<span class="rounded-full bg-accent/15 px-2 py-0.5 font-mono text-xs text-accent">{item.count}</span>
								</div>
							{/each}
						</div>
					</div>
				</div>
			{/if}
		</div>

		<!-- Distribution Row -->
		<div class="grid gap-6 sm:grid-cols-2">
			<!-- Rolls by Format -->
			{#if stats.rolls_by_format.length > 0}
				{@const maxFormat = Math.max(...stats.rolls_by_format.map((f) => f.count), 1)}
				<div>
					<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Rolls by Format</h2>
					<div class="rounded-lg border border-border bg-surface-raised p-4">
						<div class="space-y-2">
							{#each stats.rolls_by_format as item}
								<div class="flex items-center gap-3">
									<span class="w-20 text-right text-xs text-text-muted">{item.label}</span>
									<div class="flex-1">
										<div
											class="h-5 rounded bg-accent/60 transition-all duration-300"
											style="width: {(item.count / maxFormat) * 100}%"
										></div>
									</div>
									<span class="w-8 font-mono text-xs text-text-faint">{item.count}</span>
								</div>
							{/each}
						</div>
					</div>
				</div>
			{/if}

			<!-- Rolls by Status -->
			{#if stats.rolls_by_status.length > 0}
				{@const maxStatus = Math.max(...stats.rolls_by_status.map((s) => s.count), 1)}
				<div>
					<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Rolls by Status</h2>
					<div class="rounded-lg border border-border bg-surface-raised p-4">
						<div class="space-y-2">
							{#each stats.rolls_by_status as item}
								<div class="flex items-center gap-3">
									<span class="w-20 text-right text-xs text-text-muted">{item.label}</span>
									<div class="flex-1">
										<div
											class="h-5 rounded bg-accent/60 transition-all duration-300"
											style="width: {(item.count / maxStatus) * 100}%"
										></div>
									</div>
									<span class="w-8 font-mono text-xs text-text-faint">{item.count}</span>
								</div>
							{/each}
						</div>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
