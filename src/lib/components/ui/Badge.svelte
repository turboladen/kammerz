<script lang="ts">
	import type { RollStatus } from '$lib/types';

	interface Props {
		status: RollStatus;
	}

	let { status }: Props = $props();

	const statusConfig: Record<RollStatus, { label: string; pillClasses: string; dotClass: string }> = {
		loaded: { label: 'Loaded', pillClasses: 'bg-status-loaded/10 text-status-loaded', dotClass: 'bg-status-loaded' },
		shooting: { label: 'Shooting', pillClasses: 'bg-status-shooting/10 text-status-shooting', dotClass: 'bg-status-shooting' },
		shot: { label: 'Shot', pillClasses: 'bg-status-shot/10 text-status-shot', dotClass: 'bg-status-shot' },
		'at-lab': { label: 'At Lab', pillClasses: 'bg-status-at-lab/10 text-status-at-lab', dotClass: 'bg-status-at-lab' },
		developing: { label: 'Developing', pillClasses: 'bg-status-developing/10 text-status-developing', dotClass: 'bg-status-developing' },
		developed: { label: 'Developed', pillClasses: 'bg-status-developed/10 text-status-developed', dotClass: 'bg-status-developed' },
		scanned: { label: 'Scanned', pillClasses: 'bg-status-scanned/10 text-status-scanned', dotClass: 'bg-status-scanned' },
		archived: { label: 'Archived', pillClasses: 'bg-status-archived/10 text-status-archived', dotClass: 'bg-status-archived' }
	};

	const config = $derived(statusConfig[status]);
</script>

<span class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium {config.pillClasses}">
	<span class="h-1.5 w-1.5 rounded-full {config.dotClass}"></span>
	{config.label}
</span>
