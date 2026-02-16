<script lang="ts">
	interface Props {
		title: string;
		description?: string;
		backHref?: string;
		backLabel?: string;
		children?: import('svelte').Snippet;
	}

	let { title, description, backHref, backLabel, children }: Props = $props();
</script>

<div class="flex items-center justify-between border-b border-border-subtle px-6 py-4">
	<div>
		{#if backHref}
			<a href={backHref} class="mb-1 inline-flex items-center gap-1 text-xs text-text-muted transition-colors hover:text-accent">
				&larr; {backLabel ?? 'Back'}
			</a>
		{/if}
		<h1 class="font-display text-xl">{title}</h1>
		{#if description}
			<p class="mt-1 text-sm text-text-faint">{description}</p>
		{/if}
	</div>
	{#if children}
		<div class="flex items-center gap-2">
			{@render children()}
		</div>
	{/if}
</div>
