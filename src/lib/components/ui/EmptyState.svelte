<script lang="ts">
	interface Props {
		message: string;
		children?: import('svelte').Snippet;
	}

	let { message, children }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center gap-4 py-16 text-center">
	<p class="text-sm text-text-muted">{message}</p>
	{#if children}
		{@render children()}
	{/if}
</div>
