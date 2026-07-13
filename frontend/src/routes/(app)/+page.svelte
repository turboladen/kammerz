<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import FilmStrip from '$lib/components/ui/FilmStrip.svelte';
	import FilmLeader from '$lib/components/ui/FilmLeader.svelte';
	import FrameCounter from '$lib/components/ui/FrameCounter.svelte';
	import NegativesBadge from '$lib/components/ui/NegativesBadge.svelte';
	import { AlertTriangle, Check } from 'lucide-svelte';
	import { listRolls } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import { listLenses } from '$lib/api/lenses';
	import { listFilmStocks } from '$lib/api/film-stocks';
	import { updateLabDev } from '$lib/api/development';
	import type { RollWithDetails, Camera as CameraType, Lens, FilmStock, RollStatus } from '$lib/types';
	import { allStatusOrder, statusConfig } from '$lib/utils/status';
	import { negativesState, isNegativesPending } from '$lib/utils/negatives';
	import { todayLocal } from '$lib/utils/date';

	let rolls: RollWithDetails[] = $state([]);
	let cameras: CameraType[] = $state([]);
	let lenses: Lens[] = $state([]);
	let filmStocks: FilmStock[] = $state([]);
	let loading = $state(true);
	let error = $state('');

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
	const activeRolls = $derived(rolls.filter((r) => r.status === 'loaded' || r.status === 'shooting'));

	// Rolls in post-shooting processing (shot through post-processed)
	const processingStatuses: RollStatus[] = [
		'shot',
		'at-lab',
		'lab-done',
		'developing',
		'developed',
		'scanned',
		'post-processed'
	];
	const processingRolls = $derived.by(() => {
		return rolls
			.filter((r) => processingStatuses.includes(r.status))
			.sort((a, b) => {
				const statusDiff = processingStatuses.indexOf(a.status) - processingStatuses.indexOf(b.status);
				if (statusDiff !== 0) return statusDiff;
				return (b.date_finished ?? b.date_loaded ?? '').localeCompare(a.date_finished ?? a.date_loaded ?? '');
			});
	});

	const needsAttention = $derived(rolls.filter((r) => !r.camera_id && r.status !== 'archived'));

	// Rolls whose negatives are still at the lab (awaiting or overdue), each with
	// its live view. Sorted ascending by daysLeft → most-overdue first, then
	// soonest deadline (overdue has negative daysLeft).
	const negativesPending = $derived.by(() => {
		// One `now` for the whole pass so every roll's awaiting/overdue split is
		// evaluated against the same instant (and to avoid an allocation per roll).
		const now = new Date();
		return rolls
			.map((roll) => ({ roll, view: negativesState(roll, now) }))
			.filter((x) => isNegativesPending(x.view))
			.sort((a, b) => (a.view.daysLeft ?? 0) - (b.view.daysLeft ?? 0));
	});
	const negativesOverdueCount = $derived(negativesPending.filter((x) => x.view.status === 'overdue').length);
	const hasOverdue = $derived(negativesOverdueCount > 0);

	async function pickUpFromDashboard(rollLabDevId: number | null) {
		if (rollLabDevId == null) return;
		try {
			await updateLabDev(rollLabDevId, { date_negatives_picked_up: todayLocal() });
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	// Status distribution for the progress bar (uses shared allStatusOrder + statusConfig)
	const statusSegments = $derived(
		allStatusOrder
			.map((key) => ({
				key,
				label: statusConfig[key].label,
				colorVar: statusConfig[key].colorVar,
				count: rollsByStatus[key] ?? 0,
				pct: rolls.length > 0 ? ((rollsByStatus[key] ?? 0) / rolls.length) * 100 : 0
			}))
			.filter((s) => s.count > 0)
	);

	async function load() {
		try {
			[rolls, cameras, lenses, filmStocks] = await Promise.all([
				listRolls(),
				listCameras(),
				listLenses(),
				listFilmStocks()
			]);
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

<PageHeader title="Dashboard" description="Your film photography catalog at a glance">
	<Button variant="primary" href="/rolls/new">+ New Roll</Button>
</PageHeader>

<div class="flex-1 p-6">
	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if rolls.length === 0}
		<!-- Catalog stats -->
		<FadeIn>
			<div class="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<p class="font-mono text-2xl font-semibold">0</p>
					<p class="text-xs text-text-faint">Total Rolls</p>
				</div>
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<p class="font-mono text-2xl font-semibold">{cameras.length}</p>
					<p class="text-xs text-text-faint">Cameras</p>
				</div>
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<p class="font-mono text-2xl font-semibold">{lenses.length}</p>
					<p class="text-xs text-text-faint">Lenses</p>
				</div>
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<p class="font-mono text-2xl font-semibold">{filmStocks.length}</p>
					<p class="text-xs text-text-faint">Film Stocks</p>
				</div>
			</div>
		</FadeIn>

		<!-- Empty state CTA -->
		<FadeIn delay={50}>
			<div class="flex flex-col items-center gap-6 py-16 text-center">
				<FilmLeader />
				<div>
					<h2 class="font-display text-2xl text-text">Start your log</h2>
					<p class="mt-2 max-w-sm text-sm text-text-muted">
						Add your cameras, pick your film stocks, and create your first roll to begin tracking your film photography.
					</p>
				</div>
				<div class="flex gap-3">
					<Button href="/cameras">Add Cameras</Button>
					<Button variant="primary" href="/rolls/new">Create Roll</Button>
				</div>
			</div>
		</FadeIn>
	{:else}
		{#snippet rollCard(roll: RollWithDetails)}
			<a
				href="/rolls/{roll.id}?from=dashboard"
				class="group relative flex h-full flex-col overflow-hidden rounded-lg border border-border bg-surface-raised px-3.5 py-4 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
			>
				<FilmStrip />
				<div class="flex items-center gap-2">
					<span class="font-mono text-sm font-semibold">{roll.roll_id}</span>
					<Badge status={roll.status} />
					<span class="ml-auto"><FrameCounter current={roll.shot_count} total={roll.frame_count} /></span>
				</div>
				<div class="mt-1 text-xs text-text-muted">
					{#if roll.camera_brand}
						<span>{roll.camera_brand} {roll.camera_model}</span>
					{/if}
					{#if roll.camera_brand && roll.film_stock_brand}
						<span class="mx-1 text-text-faint/60">&middot;</span>
					{/if}
					{#if roll.film_stock_brand}
						<span>{roll.film_stock_brand} {roll.film_stock_name}</span>
					{/if}
				</div>
				{#if roll.film_stock_iso}
					<div class="mt-1 font-mono text-[11px] text-text-faint">ISO {roll.film_stock_iso}</div>
				{/if}
			</a>
		{/snippet}

		<!-- Negatives to Collect — the app's most time-sensitive list (rolls are
		     discarded by the lab after its retention window). Pinned to the top and
		     styled as an urgent alert panel; red when any are overdue, amber otherwise. -->
		{#if negativesPending.length > 0}
			<FadeIn>
				<section
					class="mb-8 rounded-lg border {hasOverdue
						? 'border-danger/50 bg-danger/10'
						: 'border-accent/40 bg-accent/10'}"
				>
					<header class="flex items-center justify-between gap-3 border-b border-border-subtle px-4 py-3">
						<h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">Negatives to Collect</h2>
						<span class="font-mono text-xs {hasOverdue ? 'text-danger-fg' : 'text-text-muted'}">
							{negativesPending.length}
							{negativesPending.length === 1 ? 'roll' : 'rolls'}{hasOverdue
								? ` · ${negativesOverdueCount} overdue`
								: ''}
						</span>
					</header>
					<div class="divide-y divide-border-subtle">
						{#each negativesPending as { roll, view } (roll.id)}
							<div class="flex flex-wrap items-center gap-3 px-4 py-2.5">
								<Button size="sm" variant="secondary" onclick={() => pickUpFromDashboard(roll.lab_dev_id)}>
									<Check size={14} strokeWidth={2} aria-hidden="true" /> Picked up
								</Button>
								<a href="/rolls/{roll.id}?from=dashboard" class="font-mono text-sm text-text hover:text-accent"
									>{roll.roll_id}</a
								>
								{#if roll.film_stock_brand}
									<span class="text-sm text-text-muted">{roll.film_stock_brand} {roll.film_stock_name ?? ''}</span>
								{/if}
								{#if roll.lab_name}
									<span class="text-sm text-text-faint">{roll.lab_name}</span>
								{/if}
								<span class="ml-auto"><NegativesBadge {view} /></span>
							</div>
						{/each}
					</div>
				</section>
			</FadeIn>
		{/if}

		<!-- Currently Shooting -->
		{#if activeRolls.length > 0}
			<FadeIn delay={50}>
				<div class="mb-8">
					<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
						In the Field
						<div class="flex-1 border-b border-border-subtle"></div>
					</h2>
					<div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-2.5">
						{#each activeRolls as roll}
							{@render rollCard(roll)}
						{/each}
					</div>
				</div>
			</FadeIn>
		{/if}

		<!-- In the Darkroom -->
		{#if processingRolls.length > 0}
			<FadeIn delay={100}>
				<div class="mb-8">
					<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
						In the Darkroom
						<div class="flex-1 border-b border-border-subtle"></div>
					</h2>
					<div class="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-2.5">
						{#each processingRolls as roll}
							{@render rollCard(roll)}
						{/each}
					</div>
				</div>
			</FadeIn>
		{/if}

		<!-- Quick Stats Row -->
		<FadeIn delay={150}>
			<div class="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<p class="font-mono text-2xl font-semibold">{rolls.length}</p>
					<p class="text-xs text-text-faint">Total Rolls</p>
				</div>
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<p class="font-mono text-2xl font-semibold">{cameras.length}</p>
					<p class="text-xs text-text-faint">Cameras</p>
				</div>
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<p class="font-mono text-2xl font-semibold">{activeRolls.length}</p>
					<p class="text-xs text-text-faint">In the Field</p>
				</div>
				<div class="rounded-lg border border-border bg-surface-raised p-4">
					<p class="font-mono text-2xl font-semibold">{processingRolls.length}</p>
					<p class="text-xs text-text-faint">In the Darkroom</p>
				</div>
			</div>
		</FadeIn>

		<!-- Status Distribution Bar -->
		{#if statusSegments.length > 0}
			<FadeIn delay={200}>
				<div class="mb-8">
					<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
						Roll Pipeline
						<div class="flex-1 border-b border-border-subtle"></div>
					</h2>
					<!-- Bar -->
					<div class="animate-pipeline mb-3 flex h-4 overflow-hidden rounded-full bg-surface-overlay">
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
			</FadeIn>
		{/if}

		<!-- Needs Attention -->
		{#if needsAttention.length > 0}
			<FadeIn delay={250}>
				<div>
					<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
						Needs Attention
						<div class="flex-1 border-b border-border-subtle"></div>
					</h2>
					<div class="space-y-2">
						{#each needsAttention as roll}
							<a
								href="/rolls/{roll.id}?from=dashboard"
								class="flex items-center justify-between rounded-lg border border-border bg-surface-raised p-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
							>
								<div class="flex items-center gap-3">
									<AlertTriangle size={14} class="text-status-at-lab" />
									<span class="font-mono text-sm">{roll.roll_id}</span>
									<Badge status={roll.status} />
								</div>
								<div class="text-xs text-text-muted">No camera assigned</div>
							</a>
						{/each}
					</div>
				</div>
			</FadeIn>
		{/if}
	{/if}
</div>
