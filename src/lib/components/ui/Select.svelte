<script lang="ts">
	import type { HTMLSelectAttributes } from 'svelte/elements';

	interface Props extends HTMLSelectAttributes {
		label?: string;
		options: { value: string; label: string; disabled?: boolean }[];
		placeholder?: string;
	}

	let { label, options, placeholder, value = $bindable(''), class: className, ...rest }: Props = $props();
</script>

<label class="flex flex-col gap-1.5">
	{#if label}
		<span class="text-xs font-medium text-text-muted">{label}</span>
	{/if}
	<select
		bind:value
		class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text
			transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50
			{className ?? ''}"
		{...rest}
	>
		{#if placeholder}
			<option value="" disabled>{placeholder}</option>
		{/if}
		{#each options as opt}
			<option value={opt.value} disabled={opt.disabled}>{opt.label}</option>
		{/each}
	</select>
</label>
