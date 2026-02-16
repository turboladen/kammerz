<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import { Camera, AlertTriangle, Clock } from 'lucide-svelte';
	import { listRolls } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import type { RollWithDetails, Camera as CameraType, RollStatus } from '$lib/types';

	let rolls: RollWithDetails[] = $state([]);
	let cameras: CameraType[] = $state([]);
	let loading = $state(true);

	const rollsByStatus = $derived(
		rolls.reduce(
			(acc, roll) => {
				acc[roll.status] = (acc[roll.status] || 0) + 1;
				return acc;
			},
			{} as Record<RollStatus, number>
		)
	);

	// Rolls currently in cameras (loaded or shooting)
	const activeRolls = $derived(
		rolls.filter((r) => r.status === 'loaded' || r.status === 'shooting')
	);

	const needsAttention = $derived(
		rolls.filter((r) => !r.camera_id || r.status === 'at-lab')
	);

	// Status distribution for the progress bar
	const statusOrder: { key: RollStatus; label: string; colorVar: string }[] = [
		{ key: 'loaded', label: 'Loaded', colorVar: 'var(--color-status-loaded)' },
		{ key: 'shooting', label: 'Shooting', colorVar: 'var(--color-status-shooting)' },
		{ key: 'shot', label: 'Shot', colorVar: 'var(--color-status-shot)' },
		{ key: 'at-lab', label: 'At Lab', colorVar: 'var(--color-status-at-lab)' },
		{ key: 'developing', label: 'Developing', colorVar: 'var(--color-status-developing)' },
		{ key: 'developed', label: 'Developed', colorVar: 'var(--color-status-developed)' },
		{ key: 'scanned', label: 'Scanned', colorVar: 'var(--color-status-scanned)' },
		{ key: 'archived', label: 'Archived', colorVar: 'var(--color-status-archived)' }
	];

	const statusSegments = $derived(
		statusOrder
			.map((s) => ({
				...s,
				count: rollsByStatus[s.key] ?? 0,
				pct: rolls.length > 0 ? ((rollsByStatus[s.key] ?? 0) / rolls.length) * 100 : 0
			}))
			.filter((s) => s.count > 0)
	);

	async function load() {
		try {
			[rolls, cameras] = await Promise.all([listRolls(), listCameras()]);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="Dashboard" description="Your film photography catalog at a glance">
	<Button variant="primary" href="/rolls/new">+ New Roll</Button>
</PageHeader>

<div class="flex-1 p-6">
	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if rolls.length === 0}
		<!-- Empty state for new users -->
		<div class="flex flex-col items-center gap-6 py-20 text-center">
			<div class="rounded-full bg-accent/10 p-4">
				<Camera size={32} class="text-accent" />
			</div>
			<div>
				<h2 class="font-display text-2xl text-text">Start your log</h2>
				<p class="mt-2 max-w-sm text-sm text-text-muted">Add your cameras, pick your film stocks, and create your first roll to begin tracking your film photography.</p>
			</div>
			<div class="flex gap-3">
				<Button href="/cameras">Add Cameras</Button>
				<Button variant="primary" href="/rolls/new">Create Roll</Button>
			</div>
		</div>
	{:else}
		<!-- Currently Shooting -->
		{#if activeRolls.length > 0}
			<div class="mb-8">
				<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">In the Field</h2>
				<div class="grid gap-3 {activeRolls.length > 1 ? 'sm:grid-cols-2' : ''}">
					{#each activeRolls as roll}
						<a
							href="/rolls/{roll.id}"
							class="group rounded-lg border border-accent/20 bg-accent/5 p-4 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
						>
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-3">
									<span class="font-mono text-lg font-semibold text-accent">{roll.roll_id}</span>
									<Badge status={roll.status} />
								</div>
								<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
							</div>
							<div class="mt-2 flex flex-wrap gap-3 text-sm text-text-muted">
								{#if roll.camera_brand}
									<span>{roll.camera_brand} {roll.camera_model}</span>
								{/if}
								{#if roll.film_stock_brand}
									<span class="text-text-faint">{roll.film_stock_brand} {roll.film_stock_name}</span>
								{/if}
								{#if roll.film_stock_iso}
									<span class="font-mono text-xs text-text-faint">ISO {roll.film_stock_iso}</span>
								{/if}
							</div>
						</a>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Quick Stats Row -->
		<div class="mb-8 grid grid-cols-4 gap-4">
			<div class="rounded-lg border border-border bg-surface-raised p-4">
				<p class="font-mono text-2xl font-semibold">{rolls.length}</p>
				<p class="text-xs text-text-faint">Total Rolls</p>
			</div>
			<div class="rounded-lg border border-border bg-surface-raised p-4">
				<p class="font-mono text-2xl font-semibold">{cameras.length}</p>
				<p class="text-xs text-text-faint">Cameras</p>
			</div>
			<div class="rounded-lg border border-border bg-surface-raised p-4">
				<p class="font-mono text-2xl font-semibold">{rollsByStatus['shooting'] ?? 0}</p>
				<p class="text-xs text-text-faint">Currently Shooting</p>
			</div>
			<div class="rounded-lg border border-border bg-surface-raised p-4">
				<p class="font-mono text-2xl font-semibold">{rollsByStatus['at-lab'] ?? 0}</p>
				<p class="text-xs text-text-faint">At Lab</p>
			</div>
		</div>

		<!-- Status Distribution Bar -->
		{#if statusSegments.length > 0}
			<div class="mb-8">
				<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Roll Pipeline</h2>
				<!-- Bar -->
				<div class="mb-3 flex h-2.5 overflow-hidden rounded-full bg-surface-overlay">
					{#each statusSegments as segment}
						<div
							style="width: {segment.pct}%; background-color: {segment.colorVar}"
							class="transition-all duration-300"
							title="{segment.label}: {segment.count}"
						></div>
					{/each}
				</div>
				<!-- Legend -->
				<div class="flex flex-wrap gap-x-4 gap-y-1">
					{#each statusSegments as segment}
						<span class="flex items-center gap-1.5 text-xs text-text-muted">
							<span class="h-2 w-2 rounded-full" style="background-color: {segment.colorVar}"></span>
							{segment.label}
							<span class="font-mono text-text-faint">{segment.count}</span>
						</span>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Needs Attention -->
		{#if needsAttention.length > 0}
			<div>
				<h2 class="mb-3 text-xs font-semibold uppercase tracking-wider text-text-faint">Needs Attention</h2>
				<div class="space-y-2">
					{#each needsAttention as roll}
						<a
							href="/rolls/{roll.id}"
							class="flex items-center justify-between rounded-lg border border-border bg-surface-raised p-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
						>
							<div class="flex items-center gap-3">
								{#if !roll.camera_id}
									<AlertTriangle size={14} class="text-status-at-lab" />
								{:else}
									<Clock size={14} class="text-status-at-lab" />
								{/if}
								<span class="font-mono text-sm">{roll.roll_id}</span>
								<Badge status={roll.status} />
							</div>
							<div class="text-xs text-text-muted">
								{#if !roll.camera_id}
									No camera assigned
								{:else if roll.status === 'at-lab'}
									Waiting at lab
								{/if}
							</div>
						</a>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
