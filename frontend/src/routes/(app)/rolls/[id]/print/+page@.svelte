<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { getRollDetail } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import { listLenses } from '$lib/api/lenses';
	import { listLabs } from '$lib/api/labs';
	import { lensDisplayName } from '$lib/utils/lens';
	import { buildCameraLabels } from '$lib/utils/disambiguate';
	import { getStatusLabel } from '$lib/utils/status';
	import { secondsToMmSs } from '$lib/utils/duration';
	import type { RollWithDetails, Shot, DevelopmentLab, DevelopmentSelf, DevStage, Lens, Lab, Camera } from '$lib/types';

	const id = $derived(Number(page.params.id));

	let roll: RollWithDetails | undefined = $state();
	let shots: Shot[] = $state([]);
	let shotLensMap: Record<number, number[]> = $state({});
	let labDev: DevelopmentLab | null = $state(null);
	let selfDev: DevelopmentSelf | null = $state(null);
	let devStages: DevStage[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let labs: Lab[] = $state([]);
	let cameras: Camera[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	// Reuse the SAME composite endpoint the detail page uses, plus the reference
	// catalogs needed to render lens display names, the disambiguated camera label,
	// and the lab name. A 401 here fires the client's unauthorized handler
	// (redirect to /login), so auth is still enforced at the data-fetch level even
	// though the layout-reset bypasses the (app) layout guard.
	async function load() {
		try {
			const [detail, cams, lenses, labsList] = await Promise.all([
				getRollDetail(id),
				listCameras(),
				listLenses(),
				listLabs()
			]);
			roll = detail.roll;
			shots = detail.shots;
			labDev = detail.lab_dev;
			selfDev = detail.self_dev;
			devStages = detail.dev_stages;
			cameras = cams;
			allLenses = lenses;
			labs = labsList;

			const map: Record<number, number[]> = {};
			for (const [shotId, lensId] of detail.shot_lens_pairs) {
				(map[shotId] ??= []).push(lensId);
			}
			shotLensMap = map;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	const cameraLabels = $derived(buildCameraLabels(cameras));
	const cameraLabel = $derived.by(() => {
		if (!roll?.camera_id) return null;
		const label = cameraLabels.get(roll.camera_id);
		if (label) return label;
		// Fall back to the joined brand/model on the roll when the camera isn't in
		// the catalog list (e.g. sold and filtered out — defensive).
		if (roll.camera_brand) return `${roll.camera_brand} ${roll.camera_model ?? ''}`.trim();
		return null;
	});

	const filmStockLabel = $derived.by(() => {
		if (!roll?.film_stock_brand) return null;
		const iso = roll.film_stock_iso ? `, ISO ${roll.film_stock_iso}` : '';
		return `${roll.film_stock_brand} ${roll.film_stock_name ?? ''}${iso}`.trim();
	});

	function getLabName(labId: number | null): string {
		if (!labId) return '';
		return labs.find((l) => l.id === labId)?.name ?? '';
	}

	// Per-shot lens display: per-shot override > roll default lens > none.
	function shotLensDisplay(shotId: number): string {
		const ids = shotLensMap[shotId] ?? [];
		if (ids.length > 0) {
			return ids
				.map((lid) => allLenses.find((l) => l.id === lid))
				.filter(Boolean)
				.map((l) => lensDisplayName(l!))
				.join(', ');
		}
		if (roll?.lens_id) {
			const def = allLenses.find((l) => l.id === roll?.lens_id);
			if (def) return lensDisplayName(def);
		}
		return '';
	}

	// Cost summary: the catalog records cost on the lab development record only
	// (self development has no cost field). Shown once, at the top — the
	// Development section deliberately omits it to avoid printing the value twice.
	// The local binding + $derived.by avoids a Svelte narrowing quirk where a plain
	// $derived(labDev?.cost) narrows the runed $state union to `never`.
	const totalCost: number | null = $derived.by(() => {
		const dev = labDev;
		return dev ? dev.cost : null;
	});

	// The summary page never opens the print dialog on its own — navigating here
	// (from the detail page's "Print summary" button, a bookmark, or browser
	// back/forward) just renders the page so it can be read, copied, or printed
	// on demand. The visible Print button is the only path to the dialog
	// (kammerz-k79, superseding the old ?autoprint=1 auto-open from kammerz-9qb).
	onMount(load);
</script>

<svelte:head>
	<title>{roll ? `Roll ${roll.roll_id} — Summary` : 'Roll Summary'}</title>
</svelte:head>

<div class="print-page mx-auto max-w-3xl p-8">
	<!-- Screen-only action bar (hidden when printing) -->
	<div class="no-print mb-6 flex items-center justify-between gap-3">
		<a
			href="/rolls/{id}"
			class="inline-flex items-center gap-1 text-sm text-text-muted transition-colors hover:text-accent"
		>
			&larr; Back to roll
		</a>
		<button
			onclick={() => window.print()}
			class="inline-flex items-center justify-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-surface transition-colors hover:bg-accent-hover"
		>
			Print
		</button>
	</div>

	{#if loading}
		<p class="text-sm text-text-faint">Loading…</p>
	{:else if error}
		<p class="text-sm text-red-400">{error}</p>
	{:else if !roll}
		<p class="text-sm text-text-faint">Roll not found.</p>
	{:else}
		<!-- Title -->
		<header class="mb-6 border-b border-border pb-4">
			<div class="flex items-baseline justify-between gap-4">
				<h1 class="font-mono text-2xl font-semibold">{roll.roll_id}</h1>
				<span class="text-sm text-text-muted">{getStatusLabel(roll.status)}</span>
			</div>
			<div class="mt-2 flex flex-wrap gap-x-6 gap-y-1 text-sm text-text-muted">
				{#if cameraLabel}<span>{cameraLabel}</span>{/if}
				{#if filmStockLabel}<span>{filmStockLabel}</span>{/if}
				{#if roll.push_pull}
					<span>{roll.push_pull.startsWith('+') ? 'Push' : 'Pull'} {roll.push_pull}</span>
				{/if}
				{#if roll.frame_count}<span>{roll.frame_count} frames</span>{/if}
			</div>
		</header>

		<!-- Metadata + cost summary -->
		<section class="mb-6">
			<h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-text-faint">Details</h2>
			<dl class="grid grid-cols-2 gap-x-6 gap-y-1.5 text-sm sm:grid-cols-3">
				{#if roll.date_loaded}
					<div>
						<dt class="text-text-faint">Loaded</dt>
						<dd class="font-mono">{roll.date_loaded}</dd>
					</div>
				{/if}
				{#if roll.date_finished}
					<div>
						<dt class="text-text-faint">Finished</dt>
						<dd class="font-mono">{roll.date_finished}</dd>
					</div>
				{/if}
				{#if roll.date_scanned}
					<div>
						<dt class="text-text-faint">Scanned</dt>
						<dd class="font-mono">{roll.date_scanned}</dd>
					</div>
				{/if}
				{#if roll.date_post_processed}
					<div>
						<dt class="text-text-faint">Post-processed</dt>
						<dd class="font-mono">{roll.date_post_processed}</dd>
					</div>
				{/if}
				{#if roll.date_archived}
					<div>
						<dt class="text-text-faint">Archived</dt>
						<dd class="font-mono">{roll.date_archived}</dd>
					</div>
				{/if}
				{#if roll.date_fuzzy}
					<div>
						<dt class="text-text-faint">Approx. date</dt>
						<dd>{roll.date_fuzzy}</dd>
					</div>
				{/if}
				{#if totalCost != null}
					<div>
						<dt class="text-text-faint">Cost</dt>
						<dd class="font-mono">${totalCost.toFixed(2)}</dd>
					</div>
				{/if}
			</dl>
			{#if roll.notes}
				<p class="mt-3 text-sm whitespace-pre-wrap text-text-muted">{roll.notes}</p>
			{/if}
		</section>

		<!-- Development -->
		{#if labDev || selfDev}
			<section class="mb-6">
				<h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-text-faint">Development</h2>
				{#if labDev}
					<p class="mb-1 text-sm font-medium">Lab Development</p>
					<dl class="grid grid-cols-2 gap-x-6 gap-y-1.5 text-sm sm:grid-cols-3">
						{#if labDev.lab_id && getLabName(labDev.lab_id)}
							<div>
								<dt class="text-text-faint">Lab</dt>
								<dd>{getLabName(labDev.lab_id)}</dd>
							</div>
						{/if}
						{#if labDev.date_dropped_off}
							<div>
								<dt class="text-text-faint">Submitted</dt>
								<dd class="font-mono">{labDev.date_dropped_off}</dd>
							</div>
						{/if}
						{#if labDev.date_received}
							<div>
								<dt class="text-text-faint">Received</dt>
								<dd class="font-mono">{labDev.date_received}</dd>
							</div>
						{/if}
					</dl>
					{#if labDev.notes}
						<p class="mt-2 text-sm whitespace-pre-wrap text-text-muted">{labDev.notes}</p>
					{/if}
				{:else if selfDev}
					<p class="mb-1 text-sm font-medium">Self Development</p>
					<dl class="grid grid-cols-2 gap-x-6 gap-y-1.5 text-sm sm:grid-cols-3">
						{#if selfDev.date_processed}
							<div>
								<dt class="text-text-faint">Processed</dt>
								<dd class="font-mono">{selfDev.date_processed}</dd>
							</div>
						{/if}
						{#if selfDev.temperature}
							<div>
								<dt class="text-text-faint">Temperature</dt>
								<dd>{selfDev.temperature}</dd>
							</div>
						{/if}
						{#if selfDev.developer}
							<div>
								<dt class="text-text-faint">Developer</dt>
								<dd>{selfDev.developer}{selfDev.developer_dilution ? ` (${selfDev.developer_dilution})` : ''}</dd>
							</div>
						{/if}
						{#if selfDev.fixer}
							<div>
								<dt class="text-text-faint">Fixer</dt>
								<dd>{selfDev.fixer}{selfDev.fixer_dilution ? ` (${selfDev.fixer_dilution})` : ''}</dd>
							</div>
						{/if}
						{#if selfDev.stop_bath}
							<div>
								<dt class="text-text-faint">Stop bath</dt>
								<dd>{selfDev.stop_bath}</dd>
							</div>
						{/if}
						{#if selfDev.clearing_agent}
							<div>
								<dt class="text-text-faint">Clearing agent</dt>
								<dd>{selfDev.clearing_agent}</dd>
							</div>
						{/if}
						{#if selfDev.wetting_agent}
							<div>
								<dt class="text-text-faint">Wetting agent</dt>
								<dd>{selfDev.wetting_agent}</dd>
							</div>
						{/if}
					</dl>
					{#if selfDev.agitation_notes}
						<p class="mt-2 text-sm text-text-muted">
							<span class="text-text-faint">Agitation:</span>
							{selfDev.agitation_notes}
						</p>
					{/if}
					{#if devStages.length > 0}
						<div class="mt-3">
							<p class="mb-1 text-xs font-semibold uppercase tracking-wider text-text-faint">Stages</p>
							<ol class="space-y-0.5 text-sm">
								{#each devStages as stage, i}
									<li class="flex flex-wrap items-baseline gap-x-3">
										<span class="font-mono text-text-faint">{i + 1}.</span>
										<span>{stage.stage_name}</span>
										{#if stage.duration_seconds}
											<span class="font-mono text-text-muted">{secondsToMmSs(stage.duration_seconds)}</span>
										{/if}
										{#if stage.notes}<span class="text-text-muted italic">{stage.notes}</span>{/if}
									</li>
								{/each}
							</ol>
						</div>
					{/if}
					{#if selfDev.notes}
						<p class="mt-2 text-sm whitespace-pre-wrap text-text-muted">{selfDev.notes}</p>
					{/if}
				{/if}
			</section>
		{/if}

		<!-- Shots -->
		<section>
			<h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-text-faint">
				Shots{shots.length ? ` (${shots.length})` : ''}
			</h2>
			{#if shots.length === 0}
				<p class="text-sm text-text-faint">No shots logged.</p>
			{:else}
				<table class="w-full border-collapse font-mono text-xs">
					<thead>
						<tr class="border-b border-border text-left text-text-faint">
							<th class="py-1.5 pr-3 font-medium">#</th>
							<th class="py-1.5 pr-3 font-medium">Lens</th>
							<th class="py-1.5 pr-3 font-medium">f/</th>
							<th class="py-1.5 pr-3 font-medium">Shutter</th>
							<th class="py-1.5 pr-3 font-medium">Time</th>
							<th class="py-1.5 pr-3 font-medium">Location</th>
							<th class="py-1.5 font-medium">Notes</th>
						</tr>
					</thead>
					<tbody>
						{#each shots as shot}
							<tr class="border-b border-border-subtle align-top">
								<td class="py-1.5 pr-3">{shot.frame_number}</td>
								<td class="py-1.5 pr-3">{shotLensDisplay(shot.id)}</td>
								<td class="py-1.5 pr-3">{shot.aperture ? `f/${shot.aperture}` : ''}</td>
								<td class="py-1.5 pr-3">{shot.shutter_speed ?? ''}</td>
								<td class="py-1.5 pr-3">{shot.time ?? ''}</td>
								<td class="py-1.5 pr-3">{shot.location ?? ''}</td>
								<td class="py-1.5">{shot.notes ?? ''}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			{/if}
		</section>
	{/if}
</div>

<style>
	/* The app is a dark theme; for paper output force a clean white background and
	   dark text so the PDF is legible and doesn't dump ink. The layout-reset page
	   (+page@) renders without the sidebar/chrome, so these rules only need to
	   neutralize colors and hide the screen-only action bar. */
	@media print {
		:global(html),
		:global(body) {
			background: #ffffff !important;
			color: #1a1a1a !important;
		}
		.print-page {
			max-width: none;
			padding: 0;
			color: #1a1a1a;
		}
		.no-print {
			display: none !important;
		}
		/* Flatten themed text/border colors to print-safe values. */
		.print-page :global(.text-text-faint) {
			color: #555 !important;
		}
		.print-page :global(.text-text-muted) {
			color: #333 !important;
		}
		.print-page :global(.border-border),
		.print-page :global(.border-border-subtle) {
			border-color: #ccc !important;
		}
		.print-page table {
			page-break-inside: auto;
		}
		.print-page tr {
			page-break-inside: avoid;
		}
		.print-page thead {
			display: table-header-group;
		}
	}
</style>
