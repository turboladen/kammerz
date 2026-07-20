<script lang="ts">
	import { phaseTheme } from '$lib/utils/phase';

	interface Props {
		/** The roll's server-derived compound badge label (ADR-0013), e.g. "To develop",
		 *  "Scanning · Post-processing", "Done". */
		badge: string;
		/** The roll's lifecycle-phase scalar (0..=5); drives the pill color. */
		groupKey: number;
	}

	let { badge, groupKey }: Props = $props();

	const theme = $derived(phaseTheme(groupKey));
</script>

<span class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium {theme.pillClasses}">
	<span class="h-1.5 w-1.5 rounded-full {theme.dotClass}"></span>
	{badge}
</span>
