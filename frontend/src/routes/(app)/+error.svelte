<script lang="ts">
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { Aperture } from 'lucide-svelte';

	// Errors in an authenticated (app) route render HERE — inside (app)/+layout.svelte
	// — so the sidebar stays and navigation remains usable, instead of bubbling to
	// the full-page root boundary. The root +error.svelte still handles non-(app)
	// routes (login) and the print route's +page@ layout reset (kammerz-zmr,
	// follow-up to kammerz-b21). An error in the (app) layout's OWN load can't
	// render this (its chrome failed) and correctly falls through to the root.
	const status = $derived(page.status);
	const headline = $derived(status === 404 ? 'Page not found' : 'Something went wrong');
	// page.error is App.Error ({ message }); fall back when a thrown value carried none.
	const detail = $derived(
		page.error?.message ?? (status === 404 ? "That page doesn't exist." : 'An unexpected error occurred.')
	);
</script>

<svelte:head><title>{status} — Kammerz</title></svelte:head>

<PageHeader title={headline} />
<div class="p-6">
	<EmptyState message={detail}>
		{#snippet icon()}
			<Aperture size={22} />
		{/snippet}
		<div class="mt-1 flex items-center gap-3">
			<span class="font-mono text-sm text-text-faint">Error {status}</span>
			<Button href="/">Back to dashboard</Button>
		</div>
	</EmptyState>
</div>
