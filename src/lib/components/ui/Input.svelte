<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	interface Props extends HTMLInputAttributes {
		label?: string;
		hint?: string;
		error?: string;
	}

	let { label, hint, error, value = $bindable(), class: className, ...rest }: Props = $props();
</script>

<label class="flex flex-col gap-1.5">
	{#if label}
		<span class="text-xs font-medium text-text-muted">{label}</span>
	{/if}
	<input
		bind:value
		class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text placeholder-text-faint
			transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50
			{error ? 'border-red-500' : ''} {className ?? ''}"
		{...rest}
	/>
	{#if error}
		<span class="text-xs text-red-400">{error}</span>
	{:else if hint}
		<span class="text-xs text-text-faint">{hint}</span>
	{/if}
</label>
