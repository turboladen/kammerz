<script lang="ts">
	import type { HTMLTextareaAttributes } from 'svelte/elements';

	interface Props extends HTMLTextareaAttributes {
		label?: string;
	}

	let { label, value = $bindable(), class: className, ...rest }: Props = $props();
</script>

<label class="flex flex-col gap-1.5">
	{#if label}
		<span class="text-xs font-medium text-text-muted">{label}</span>
	{/if}
	<textarea
		bind:value
		class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text placeholder-text-faint
			transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50
			{className ?? ''}"
		rows="3"
		{...rest}
	></textarea>
</label>
