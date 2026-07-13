<script lang="ts">
	import type { NegativesView } from '$lib/utils/negatives';

	let { view }: { view: NegativesView } = $props();

	// Tier → pill classes, using existing theme tokens (accent = amber, danger = red).
	const tierClasses: Record<NegativesView['tier'], string> = {
		none: '',
		far: 'bg-surface-overlay text-text-faint',
		near: 'bg-accent/10 text-accent',
		soon: 'bg-accent/20 text-accent',
		overdue: 'bg-danger/15 text-danger-fg'
	};
</script>

{#if view.label}
	<span
		class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium {tierClasses[view.tier]}"
		title="Negatives to collect"
	>
		{view.label}
	</span>
{/if}
