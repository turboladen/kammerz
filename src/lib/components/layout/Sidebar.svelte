<script lang="ts">
	import { page } from '$app/state';
	import { LayoutDashboard, Search, Film, Camera, Aperture, Package, FlaskConical, BarChart3, Plus } from 'lucide-svelte';

	const navItems = [
		{ href: '/', label: 'Dashboard', icon: LayoutDashboard },
		{ href: '/search', label: 'Search', icon: Search },
		{ href: '/rolls', label: 'Rolls', icon: Film },
		{ href: '/cameras', label: 'Cameras', icon: Camera },
		{ href: '/lenses', label: 'Lenses', icon: Aperture },
		{ href: '/film-stocks', label: 'Film Stocks', icon: Package },
		{ href: '/labs', label: 'Labs', icon: FlaskConical },
		{ href: '/stats', label: 'Stats', icon: BarChart3 }
	];

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname.startsWith(href);
	}
</script>

<nav class="flex h-full w-56 flex-col border-r border-border-subtle bg-gradient-to-b from-surface-raised to-surface">
	<!-- Brand area -->
	<div class="border-b border-border-subtle px-5 py-4">
		<span class="font-display text-xl tracking-wide text-accent">Kamerz</span>
		<p class="mt-0.5 font-mono text-[10px] uppercase tracking-widest text-text-faint">film log</p>
	</div>

	<!-- Navigation -->
	<div class="flex flex-1 flex-col gap-0.5 p-3">
		{#each navItems as item}
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
</nav>
