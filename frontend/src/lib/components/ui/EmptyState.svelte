<script lang="ts">
	interface Props {
		message: string;
		title?: string;
		icon?: import('svelte').Snippet;
		children?: import('svelte').Snippet;
	}

	let { message, title, icon, children }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center gap-4 py-16 text-center">
	{#if icon}
		<div class="flex h-12 w-12 items-center justify-center rounded-full bg-accent/10 text-accent">
			{@render icon()}
		</div>
	{/if}
	{#if title}
		<p class="font-display text-lg text-text">{title}</p>
	{/if}
	<p class="text-sm text-text-muted">{message}</p>
	{#if children}
		{@render children()}
	{/if}
</div>
