<script lang="ts">
	import { page } from '$app/state';
	import Button from '$lib/components/ui/Button.svelte';
	import { Aperture } from 'lucide-svelte';

	// SvelteKit renders this for ANY uncaught load/render error in the route tree
	// when no nearer +error.svelte exists. It is the catch-all root boundary, so it
	// also covers the print summary route — a +page@ layout reset bottoms out at
	// the ROOT layout, bypassing the (app) boundary. Replaces SvelteKit's bare,
	// unstyled "500 Internal Error" fallback with the app theme (kammerz-b21).
	const status = $derived(page.status);
	const headline = $derived(status === 404 ? 'Page not found' : 'Something went wrong');
	// page.error is App.Error ({ message }); fall back when a thrown value carried none.
	const detail = $derived(
		page.error?.message ?? (status === 404 ? "That page doesn't exist." : 'An unexpected error occurred.')
	);
</script>

<svelte:head><title>{status} — Kammerz</title></svelte:head>

<!-- No bg-surface: let the html canvas (surface color + vignette) show through so
     the error screen carries the same photographic vignette as the app pages,
     mirroring the login layout. -->
<div class="flex min-h-screen items-center justify-center px-4">
	<div class="w-full max-w-md rounded-xl border border-border-subtle bg-surface-raised p-8 text-center shadow-2xl">
		<span class="mx-auto mb-5 flex h-12 w-12 items-center justify-center rounded-full bg-accent/15 text-accent">
			<Aperture size={24} />
		</span>
		<p class="font-mono text-4xl font-semibold text-text">{status}</p>
		<h1 class="mt-2 font-display text-lg text-text">{headline}</h1>
		<p class="mt-2 text-sm text-text-muted">{detail}</p>
		<div class="mt-6 flex justify-center">
			<Button href="/">Back to dashboard</Button>
		</div>
	</div>
</div>
