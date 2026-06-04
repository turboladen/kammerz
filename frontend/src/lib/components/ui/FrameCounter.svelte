<script lang="ts">
	// Mechanical frame-counter plaque — a supporting film motif. Shows `current/total`
	// in tabular mono, like a camera's frame window. `lg` is the roll-detail hero
	// plaque (with a "frames" caption); `sm` is a compact chip for dashboard/list
	// cards. Renders nothing when there's no data to show. Over-count (more shots than
	// the roll's frame_count) is flagged with the existing danger foreground token —
	// no new colors introduced.
	interface Props {
		current?: number;
		total?: number | null;
		size?: 'sm' | 'lg';
	}

	let { current, total, size = 'sm' }: Props = $props();

	const hasTotal = $derived(total != null);
	const hasCurrent = $derived(current != null);
	const show = $derived(hasTotal || hasCurrent);
	const over = $derived(current != null && total != null && current > total);
	const numberColor = $derived(
		over ? 'text-danger-fg' : size === 'lg' ? 'text-text' : 'text-text-muted'
	);

	const ariaLabel = $derived(
		hasCurrent && hasTotal
			? `${current} of ${total} frames`
			: hasTotal
				? `${total} frame capacity`
				: `${current} frames`
	);
</script>

<!-- Shared count readout for both sizes. Within {#if show} at least one of
     current/total exists, so: both → `current/total`; current only → `current`;
     total only → just the capacity `total`. The `/total` suffix is only added when
     both are present, so we never render a dangling `/36` or `·/36`. -->
{#snippet readout(numberClass: string)}
	<span class="{numberClass} {numberColor}">{hasCurrent ? current : total}{#if hasCurrent && hasTotal}<span class="text-text-faint">/{total}</span>{/if}</span>
{/snippet}

{#if show}
	{#if size === 'lg'}
		<span
			class="inline-flex flex-col items-center rounded-md border border-border bg-surface px-3 py-1.5 tabular-nums"
			aria-label={ariaLabel}
		>
			{@render readout('font-mono text-xl font-semibold leading-none')}
			<span class="mt-1 text-[9px] font-medium uppercase tracking-widest text-text-faint/70">frames</span>
		</span>
	{:else}
		<span
			class="inline-flex items-center rounded border border-border-subtle bg-surface px-1.5 py-0.5 font-mono text-[11px] tabular-nums"
			aria-label={ariaLabel}
		>
			{@render readout('')}
		</span>
	{/if}
{/if}
