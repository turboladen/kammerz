<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import {
		LayoutDashboard,
		Search,
		Film,
		Camera,
		Aperture,
		Package,
		FlaskConical,
		TestTube2,
		BarChart3,
		Plus,
		LogOut
	} from 'lucide-svelte';

	// Core data entity navigation
	const mainNavItems = [
		{ href: '/', label: 'Dashboard', icon: LayoutDashboard },
		{ href: '/rolls', label: 'Rolls', icon: Film },
		{ href: '/cameras', label: 'Cameras', icon: Camera },
		{ href: '/lenses', label: 'Lenses', icon: Aperture },
		{ href: '/film-stocks', label: 'Film Stocks', icon: Package },
		{ href: '/labs', label: 'Labs', icon: FlaskConical },
		{ href: '/developments', label: 'Developing', icon: TestTube2 }
	];

	// Utility / analytics navigation
	const utilityNavItems = [
		{ href: '/search', label: 'Search', icon: Search },
		{ href: '/stats', label: 'Stats', icon: BarChart3 }
	];

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname.startsWith(href);
	}

	// Sign-out is only meaningful when a password is configured (auth.authRequired).
	// In open LAN-trust mode there is no session to end, so the control is hidden.
	let signingOut = $state(false);
	async function signOut() {
		if (signingOut) return;
		signingOut = true;
		try {
			await auth.logout();
		} catch {
			// Swallow: a failed server logout shouldn't trap the user. The redirect
			// below sends them to /login, and the layout guard re-checks the session.
		} finally {
			await goto('/login');
			signingOut = false;
		}
	}
</script>

<nav class="flex h-full w-56 flex-col border-r border-border-subtle bg-gradient-to-b from-surface-raised to-surface">
	<!-- Brand area -->
	<div class="border-b border-border-subtle px-5 py-4">
		<span class="font-display text-xl tracking-wide text-accent">Kammerz</span>
		<p class="mt-0.5 font-mono text-[10px] uppercase tracking-widest text-text-faint">film log</p>
	</div>

	<!-- Navigation -->
	<div class="flex flex-1 flex-col p-3">
		<div class="flex flex-col gap-0.5">
			{#each mainNavItems as item}
				<a
					href={item.href}
					class="relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-150
						{isActive(item.href)
						? 'border-l-2 border-accent bg-accent/8 text-accent'
						: 'border-l-2 border-transparent text-text-muted hover:bg-surface-overlay hover:text-text'}"
				>
					<item.icon size={16} strokeWidth={1.75} class={isActive(item.href) ? 'text-accent' : 'text-text-faint'} />
					{item.label}
				</a>
			{/each}
		</div>
		<div class="mt-2 flex flex-col gap-0.5 border-t border-border-subtle pt-2">
			{#each utilityNavItems as item}
				<a
					href={item.href}
					class="relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-all duration-150
						{isActive(item.href)
						? 'border-l-2 border-accent bg-accent/8 text-accent'
						: 'border-l-2 border-transparent text-text-muted hover:bg-surface-overlay hover:text-text'}"
				>
					<item.icon size={16} strokeWidth={1.75} class={isActive(item.href) ? 'text-accent' : 'text-text-faint'} />
					{item.label}
				</a>
			{/each}
		</div>
	</div>

	<!-- Quick Entry -->
	<div class="border-t border-border-subtle p-3">
		<a
			href="/quick-entry"
			class="flex items-center justify-center gap-2 rounded-lg border border-dashed border-border px-3 py-2 text-sm font-medium text-text-muted transition-all duration-150 hover:border-accent hover:text-accent"
		>
			<Plus size={14} strokeWidth={2} />
			Quick Entry
		</a>
	</div>

	{#if auth.authRequired}
		<!-- Sign out -->
		<div class="border-t border-border-subtle p-3">
			<button
				type="button"
				onclick={signOut}
				disabled={signingOut}
				class="flex w-full items-center gap-3 rounded-lg border-l-2 border-transparent px-3 py-2 text-sm font-medium text-text-muted transition-all duration-150 hover:bg-surface-overlay hover:text-text disabled:cursor-not-allowed disabled:opacity-60"
			>
				<LogOut size={16} strokeWidth={1.75} class="text-text-faint" />
				{signingOut ? 'Signing out…' : 'Sign out'}
			</button>
		</div>
	{/if}
</nav>
