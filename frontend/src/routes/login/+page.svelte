<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth.svelte';
	import { ApiRequestError } from '$lib/api/client';
	import { safeNext } from '$lib/utils/redirect';
	import Input from '$lib/components/ui/Input.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { Aperture } from 'lucide-svelte';

	let password = $state('');
	let error = $state('');
	let submitting = $state(false);

	async function submit() {
		if (submitting) return;
		error = '';
		submitting = true;
		try {
			const ok = await auth.login(password);
			if (ok) await goto(safeNext(page.url.searchParams.get('next'), page.url.origin));
			else error = 'Incorrect password';
		} catch (e) {
			error =
				e instanceof ApiRequestError && e.status === 401
					? 'Incorrect password'
					: 'Something went wrong. Please try again.';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head><title>Sign in — Kammerz</title></svelte:head>

<!-- No bg-surface: let the html canvas (surface color + vignette) show through so
     the login screen carries the same photographic vignette as the app pages. -->
<div class="flex min-h-screen items-center justify-center px-4">
	<div class="w-full max-w-sm rounded-xl border border-border-subtle bg-surface-raised p-8 shadow-2xl">
		<div class="mb-6 flex items-center gap-3">
			<span class="flex h-10 w-10 items-center justify-center rounded-lg bg-accent/15 text-accent">
				<Aperture size={22} />
			</span>
			<h1 class="font-display text-4xl leading-none text-accent">Kammerz</h1>
		</div>

		<p class="mb-6 text-sm text-text-muted">Enter the password to access your catalog.</p>

		{#if error}
			<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}

		<div class="mb-4">
			<Input
				type="password"
				label="Password"
				bind:value={password}
				placeholder="••••••••"
				autocomplete="current-password"
				onkeydown={(e) => e.key === 'Enter' && submit()}
			/>
		</div>

		<Button variant="primary" class="w-full" onclick={submit} disabled={submitting}>
			{submitting ? 'Signing in…' : 'Sign in'}
		</Button>
	</div>
</div>
