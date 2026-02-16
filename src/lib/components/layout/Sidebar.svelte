<script lang="ts">
	import { page } from '$app/state';

	const navItems = [
		{ href: '/', label: 'Dashboard', icon: 'grid' },
		{ href: '/rolls', label: 'Rolls', icon: 'film' },
		{ href: '/cameras', label: 'Cameras', icon: 'camera' },
		{ href: '/lenses', label: 'Lenses', icon: 'aperture' },
		{ href: '/film-stocks', label: 'Film Stocks', icon: 'box' },
		{ href: '/labs', label: 'Labs', icon: 'flask' }
	];

	function isActive(href: string): boolean {
		if (href === '/') return page.url.pathname === '/';
		return page.url.pathname.startsWith(href);
	}
</script>

<nav class="flex h-full w-56 flex-col border-r border-border bg-surface-raised">
	<div class="flex items-center gap-2 border-b border-border px-5 py-4">
		<span class="text-lg font-semibold tracking-tight text-accent">Kamerz</span>
	</div>

	<div class="flex flex-1 flex-col gap-1 p-3">
		{#each navItems as item}
			<a
				href={item.href}
				class="flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors
					{isActive(item.href)
					? 'bg-accent/15 text-accent'
					: 'text-text-muted hover:bg-surface-overlay hover:text-text'}"
			>
				<span class="text-xs opacity-60">{item.icon}</span>
				{item.label}
			</a>
		{/each}
	</div>

	<div class="border-t border-border p-3">
		<a
			href="/quick-entry"
			class="flex items-center justify-center gap-2 rounded-lg bg-accent/20 px-3 py-2 text-sm font-medium text-accent transition-colors hover:bg-accent/30"
		>
			+ Quick Entry
		</a>
	</div>
</nav>
