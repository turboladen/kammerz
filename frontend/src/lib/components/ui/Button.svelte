<script lang="ts">
	import type { HTMLButtonAttributes } from 'svelte/elements';

	interface Props extends HTMLButtonAttributes {
		variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
		size?: 'sm' | 'md';
		href?: string;
	}

	let { variant = 'secondary', size = 'md', href, children, class: className, ...rest }: Props = $props();

	const base = 'inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-accent/50 disabled:opacity-50 disabled:pointer-events-none';

	const variants = {
		primary: 'bg-accent text-surface hover:bg-accent-hover',
		secondary: 'bg-surface-overlay text-text border border-border hover:bg-surface-overlay/80',
		ghost: 'text-text-muted hover:bg-surface-overlay hover:text-text',
		danger: 'bg-danger text-white hover:bg-danger-hover'
	};

	const sizes = {
		sm: 'px-2.5 py-1.5 text-xs',
		md: 'px-4 py-2 text-sm'
	};

	const classes = $derived(`${base} ${variants[variant]} ${sizes[size]} ${className ?? ''}`);
</script>

{#if href}
	<a {href} class={classes}>
		{#if children}{@render children()}{/if}
	</a>
{:else}
	<button class={classes} {...rest}>
		{#if children}{@render children()}{/if}
	</button>
{/if}
