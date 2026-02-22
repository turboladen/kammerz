<script lang="ts">
	interface Props {
		delay?: number;
		children: import('svelte').Snippet;
	}

	let { delay = 0, children }: Props = $props();

	// After the entrance animation finishes, strip the animation class so that
	// the element no longer creates a stacking context / containing block.
	// This prevents `position: fixed` children (e.g. Dialog overlays) from
	// being trapped inside the animated wrapper.
	function handleAnimationEnd(e: AnimationEvent) {
		if (e.target !== e.currentTarget) return; // ignore bubbled child animations
		const el = e.currentTarget as HTMLElement;
		el.classList.remove('animate-fade-in-up');
		el.style.removeProperty('animation-delay');
	}
</script>

<div class="animate-fade-in-up" style="animation-delay: {delay}ms" onanimationend={handleAnimationEnd}>
	{@render children()}
</div>
