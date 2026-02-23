<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import { searchCatalog } from '$lib/api/search';
	import type { SearchResults } from '$lib/types';
	import { Camera, Aperture, Package, Film, Focus, FlaskConical, Search } from 'lucide-svelte';

	let query = $state('');
	let results: SearchResults | null = $state(null);
	let loading = $state(false);
	let error = $state('');

	const totalResults = $derived(
		results
			? results.cameras.length +
				results.lenses.length +
				results.film_stocks.length +
				results.rolls.length +
				results.shots.length +
				results.labs.length
			: 0
	);

	const hasResults = $derived(totalResults > 0);

	// Debounced search via $effect
	$effect(() => {
		const q = query.trim();
		if (q.length < 2) {
			results = null;
			error = '';
			return;
		}

		const timeout = setTimeout(async () => {
			loading = true;
			error = '';
			try {
				results = await searchCatalog(q);
			} catch (err) {
				error = err instanceof Error ? err.message : String(err);
				results = null;
			} finally {
				loading = false;
			}
		}, 300);

		return () => clearTimeout(timeout);
	});
</script>

<PageHeader title="Search" description="Search across your entire catalog" />

<div class="p-6">
	<!-- Search input -->
	<div class="relative max-w-xl">
		<div class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-faint">
			<Search size={16} strokeWidth={1.75} />
		</div>
		<!-- svelte-ignore a11y_autofocus -->
		<input
			bind:value={query}
			type="text"
			placeholder="Search cameras, lenses, film stocks, rolls, shots, labs..."
			class="w-full rounded-lg border border-border bg-surface pl-10 pr-3 py-2.5 text-sm text-text placeholder-text-faint
				transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
			autofocus
		/>
	</div>

	{#if error}
		<div class="mt-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	{#if loading}
		<p class="mt-6 text-sm text-text-muted">Searching...</p>
	{:else if query.trim().length < 2}
		<p class="mt-6 text-sm text-text-faint">Type at least 2 characters to search.</p>
	{:else if results && !hasResults}
		<div class="mt-8 text-center">
			<p class="text-sm text-text-muted">No results found for "{query}"</p>
			<p class="mt-1 text-xs text-text-faint">Try a different search term</p>
		</div>
	{:else if results}
		<div class="mt-6 space-y-6">
			<!-- Cameras -->
			{#if results.cameras.length > 0}
				<FadeIn>
					<section class="border-l-2 border-accent/40 pl-4">
						<div class="mb-2 flex items-center gap-2">
							<Camera size={14} strokeWidth={1.75} class="text-accent" />
							<h2 class="text-xs font-semibold uppercase tracking-wider text-text-muted">
								Cameras <span class="text-text-faint">({results.cameras.length})</span>
							</h2>
						</div>
						<div class="grid gap-1.5">
							{#each results.cameras as cam}
								<a
									href="/cameras/{cam.id}?from=search"
									class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
								>
									<div class="flex items-center gap-3">
										<span class="text-sm font-medium">{cam.brand} {cam.model}</span>
										<span class="text-xs text-text-faint">{cam.format}</span>
									</div>
									<div class="flex items-center gap-2">
										<span class="text-xs text-text-faint">in {cam.match_field}</span>
										<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
									</div>
								</a>
							{/each}
						</div>
					</section>
				</FadeIn>
			{/if}

			<!-- Lenses -->
			{#if results.lenses.length > 0}
				<FadeIn delay={50}>
					<section class="border-l-2 border-accent/40 pl-4">
						<div class="mb-2 flex items-center gap-2">
							<Aperture size={14} strokeWidth={1.75} class="text-accent" />
							<h2 class="text-xs font-semibold uppercase tracking-wider text-text-muted">
								Lenses <span class="text-text-faint">({results.lenses.length})</span>
							</h2>
						</div>
						<div class="grid gap-1.5">
							{#each results.lenses as lens}
								<a
									href="/lenses/{lens.id}"
									class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
								>
									<div class="flex items-center gap-3">
										<span class="text-sm font-medium">{lens.model ? (lens.model.toLowerCase().startsWith(lens.brand.toLowerCase()) ? lens.model : `${lens.brand} ${lens.model}`) : lens.brand}</span>
										{#if lens.focal_length}
											<span class="text-xs text-text-faint">{lens.focal_length}</span>
										{/if}
									</div>
									<div class="flex items-center gap-2">
										<span class="text-xs text-text-faint">in {lens.match_field}</span>
										<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
									</div>
								</a>
							{/each}
						</div>
					</section>
				</FadeIn>
			{/if}

			<!-- Film Stocks -->
			{#if results.film_stocks.length > 0}
				<FadeIn delay={100}>
					<section class="border-l-2 border-accent/40 pl-4">
						<div class="mb-2 flex items-center gap-2">
							<Package size={14} strokeWidth={1.75} class="text-accent" />
							<h2 class="text-xs font-semibold uppercase tracking-wider text-text-muted">
								Film Stocks <span class="text-text-faint">({results.film_stocks.length})</span>
							</h2>
						</div>
						<div class="grid gap-1.5">
							{#each results.film_stocks as fs}
								<a
									href="/film-stocks/{fs.id}"
									class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
								>
									<div class="flex items-center gap-3">
										<span class="text-sm font-medium">{fs.brand} {fs.name}</span>
										<span class="text-xs text-text-faint">{fs.format} · {fs.stock_type}</span>
									</div>
									<div class="flex items-center gap-2">
										<span class="text-xs text-text-faint">in {fs.match_field}</span>
										<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
									</div>
								</a>
							{/each}
						</div>
					</section>
				</FadeIn>
			{/if}

			<!-- Rolls -->
			{#if results.rolls.length > 0}
				<FadeIn delay={150}>
					<section class="border-l-2 border-accent/40 pl-4">
						<div class="mb-2 flex items-center gap-2">
							<Film size={14} strokeWidth={1.75} class="text-accent" />
							<h2 class="text-xs font-semibold uppercase tracking-wider text-text-muted">
								Rolls <span class="text-text-faint">({results.rolls.length})</span>
							</h2>
						</div>
						<div class="grid gap-1.5">
							{#each results.rolls as roll}
								<a
									href="/rolls/{roll.id}?from=search"
									class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
								>
									<div class="flex items-center gap-3">
										<span class="font-mono text-sm font-semibold">{roll.roll_id}</span>
										<Badge status={roll.status} />
										{#if roll.camera_brand}
											<span class="text-xs text-text-muted">{roll.camera_brand} {roll.camera_model}</span>
										{/if}
									</div>
									<div class="flex items-center gap-2">
										{#if roll.film_stock_brand}
											<span class="text-xs text-text-faint">{roll.film_stock_brand} {roll.film_stock_name}</span>
										{/if}
										<span class="text-xs text-text-faint">· in {roll.match_field}</span>
										<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
									</div>
								</a>
							{/each}
						</div>
					</section>
				</FadeIn>
			{/if}

			<!-- Shots -->
			{#if results.shots.length > 0}
				<FadeIn delay={200}>
					<section class="border-l-2 border-accent/40 pl-4">
						<div class="mb-2 flex items-center gap-2">
							<Focus size={14} strokeWidth={1.75} class="text-accent" />
							<h2 class="text-xs font-semibold uppercase tracking-wider text-text-muted">
								Shots <span class="text-text-faint">({results.shots.length})</span>
							</h2>
						</div>
						<div class="grid gap-1.5">
							{#each results.shots as shot}
								<a
									href="/rolls/{shot.roll_pk}?from=search"
									class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
								>
									<div class="flex items-center gap-3">
										<span class="font-mono text-sm font-semibold">#{shot.frame_number}</span>
										<span class="text-xs text-text-muted">Roll {shot.roll_id_display}</span>
										{#if shot.location}
											<span class="text-xs text-text-faint">{shot.location}</span>
										{/if}
									</div>
									<div class="flex items-center gap-2">
										<span class="text-xs text-text-faint">in {shot.match_field}: {shot.match_snippet}</span>
										<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
									</div>
								</a>
							{/each}
						</div>
					</section>
				</FadeIn>
			{/if}

			<!-- Labs -->
			{#if results.labs.length > 0}
				<FadeIn delay={250}>
					<section class="border-l-2 border-accent/40 pl-4">
						<div class="mb-2 flex items-center gap-2">
							<FlaskConical size={14} strokeWidth={1.75} class="text-accent" />
							<h2 class="text-xs font-semibold uppercase tracking-wider text-text-muted">
								Labs <span class="text-text-faint">({results.labs.length})</span>
							</h2>
						</div>
						<div class="grid gap-1.5">
							{#each results.labs as lab}
								<a
									href="/labs/{lab.id}"
									class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
								>
									<div class="flex items-center gap-3">
										<span class="text-sm font-medium">{lab.name}</span>
										{#if lab.location}
											<span class="text-xs text-text-faint">{lab.location}</span>
										{/if}
									</div>
									<div class="flex items-center gap-2">
										<span class="text-xs text-text-faint">in {lab.match_field}</span>
										<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
									</div>
								</a>
							{/each}
						</div>
					</section>
				</FadeIn>
			{/if}
		</div>
	{/if}
</div>
