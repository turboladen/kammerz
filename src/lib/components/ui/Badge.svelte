<script lang="ts">
	import type { RollStatus } from '$lib/types';
	import { statusConfig } from '$lib/utils/status';

	interface Props {
		status: RollStatus;
	}

	let { status }: Props = $props();

	const config = $derived(statusConfig[status]);
</script>

<span class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium {config.pillClasses}">
	<span class="h-1.5 w-1.5 rounded-full {config.dotClass}"></span>
	{config.label}
</span>
