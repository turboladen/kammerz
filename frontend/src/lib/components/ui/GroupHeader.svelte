<script lang="ts">
	interface Props {
		label: string;
	}

	let { label }: Props = $props();
</script>

{#if label}
	<div class="mb-2 mt-5 flex items-center gap-3">
		<h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">{label}</h2>
		<div class="flex-1 border-b border-border-subtle"></div>
	</div>
{/if}
