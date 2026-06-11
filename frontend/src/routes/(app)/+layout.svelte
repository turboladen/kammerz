<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { Menu } from 'lucide-svelte';
	import Sidebar from '$lib/components/layout/Sidebar.svelte';

	let { children } = $props();

	// Mobile-only nav drawer state; on md+ the sidebar is a persistent column.
	let sidebarOpen = $state(false);

	// Close the drawer after any in-app navigation (tapping a nav link).
	afterNavigate(() => {
		sidebarOpen = false;
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') sidebarOpen = false;
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex h-screen overflow-hidden">
	<!-- Mobile drawer backdrop -->
	{#if sidebarOpen}
		<button
			class="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm md:hidden"
			aria-label="Close navigation"
			onclick={() => (sidebarOpen = false)}
		></button>
	{/if}

	<!-- Sidebar: slide-in drawer below md, persistent column on md+ -->
	<div
		class="fixed inset-y-0 left-0 z-50 transition-transform duration-200 md:static md:z-auto md:translate-x-0 md:transition-none
			{sidebarOpen ? 'translate-x-0' : '-translate-x-full'}"
	>
		<Sidebar />
	</div>

	<main class="flex min-w-0 flex-1 flex-col overflow-y-auto">
		<!-- Mobile top bar with hamburger -->
		<header
			class="flex shrink-0 items-center gap-3 border-b border-border-subtle bg-surface-raised px-4 py-3 md:hidden"
		>
			<button
				onclick={() => (sidebarOpen = true)}
				aria-label="Open navigation"
				class="rounded-lg p-1.5 text-text-muted transition-colors hover:bg-surface-overlay hover:text-text"
			>
				<Menu size={18} strokeWidth={1.75} />
			</button>
			<span class="font-display text-lg tracking-wide text-accent">Kammerz</span>
		</header>

		{@render children()}
	</main>
</div>
