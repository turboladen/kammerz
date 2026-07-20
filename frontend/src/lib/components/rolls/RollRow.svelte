<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { RollWithDetails } from '$lib/types';
	import Badge from '$lib/components/ui/Badge.svelte';
	import FilmStrip from '$lib/components/ui/FilmStrip.svelte';
	import FrameCounter from '$lib/components/ui/FrameCounter.svelte';

	interface Props {
		roll: RollWithDetails;
		/** Render as a link (rolls list). Mutually exclusive with onclick. */
		href?: string;
		/** Render as a select button (picker). Mutually exclusive with href. */
		onclick?: () => void;
		/** Accent border for the chosen/selected state (collapsed picker). */
		selected?: boolean;
		/** Right-most element: the → arrow (list) or a Change button (collapsed picker). */
		trailing?: Snippet;
	}

	let { roll, href, onclick, selected = false, trailing }: Props = $props();

	const interactive = $derived(!!href || !!onclick);
	// href → <a>, onclick → <button>, neither → static <div> (so a trailing <button>
	// inside the collapsed row isn't an invalid nested button).
	const tag = $derived(href ? 'a' : onclick ? 'button' : 'div');
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<svelte:element
	this={tag}
	{href}
	{onclick}
	type={tag === 'button' ? 'button' : undefined}
	class="group relative flex w-full items-center gap-x-3 overflow-hidden rounded-lg border bg-surface-raised py-2.5 pl-5 pr-4 text-left transition-all duration-150 {selected
		? 'border-accent'
		: 'border-border'} {interactive ? 'hover:border-accent/40 hover:-translate-y-px' : ''}"
>
	<FilmStrip orientation="vertical" />
	<span class="shrink-0 font-mono text-sm font-semibold">{roll.roll_id}</span>
	<Badge badge={roll.badge} groupKey={roll.group_key} />
	<!-- Ledger metadata: flows left-to-right with dot separators, wraps gracefully. -->
	<div class="flex flex-wrap items-center gap-x-2 gap-y-0.5">
		{#if roll.camera_brand}
			<span class="text-sm text-text-muted">{roll.camera_brand} {roll.camera_model}</span>
		{:else}
			<span class="text-sm italic text-text-faint">No camera</span>
		{/if}
		{#if roll.film_stock_brand}
			<span class="text-xs text-text-faint" aria-hidden="true">&middot;</span>
			<span class="text-xs text-text-faint">{roll.film_stock_brand} {roll.film_stock_name}</span>
		{/if}
		{#if roll.lens_brand}
			<span class="text-xs text-text-faint" aria-hidden="true">&middot;</span>
			<span class="text-xs text-text-faint">{roll.lens_brand} {roll.lens_name}</span>
		{/if}
		{#if roll.date_loaded}
			<span class="text-xs text-text-faint" aria-hidden="true">&middot;</span>
			<span class="font-mono text-xs text-text-faint">{roll.date_loaded}</span>
		{/if}
	</div>
	<!-- Right-anchored: frame counter + optional trailing element. -->
	<div class="ml-auto flex shrink-0 items-center gap-2.5">
		<FrameCounter current={roll.shot_count} total={roll.frame_count} />
		{#if trailing}{@render trailing()}{/if}
	</div>
</svelte:element>
