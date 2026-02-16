<script lang="ts">
	import type { RollStatus } from '$lib/types';

	interface Props {
		status: RollStatus;
	}

	let { status }: Props = $props();

	const statusConfig: Record<RollStatus, { label: string; classes: string }> = {
		loaded: { label: 'Loaded', classes: 'bg-status-loaded/20 text-status-loaded' },
		shooting: { label: 'Shooting', classes: 'bg-status-shooting/20 text-status-shooting' },
		shot: { label: 'Shot', classes: 'bg-status-shot/20 text-status-shot' },
		'at-lab': { label: 'At Lab', classes: 'bg-status-at-lab/20 text-status-at-lab' },
		developing: { label: 'Developing', classes: 'bg-status-developing/20 text-status-developing' },
		developed: { label: 'Developed', classes: 'bg-status-developed/20 text-status-developed' },
		scanned: { label: 'Scanned', classes: 'bg-status-scanned/20 text-status-scanned' },
		archived: { label: 'Archived', classes: 'bg-status-archived/20 text-status-archived' }
	};

	const config = $derived(statusConfig[status]);
</script>

<span class="inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium {config.classes}">
	{config.label}
</span>
