<script lang="ts">
	interface Props {
		delay?: number;
		children: import('svelte').Snippet;
	}

	let { delay = 0, children }: Props = $props();
</script>

<div class="animate-fade-in-up" style="animation-delay: {delay}ms">
	{@render children()}
</div>
