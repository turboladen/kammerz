<script lang="ts">
	interface Props {
		open: boolean;
		title: string;
		children: import('svelte').Snippet;
		onclose?: () => void;
	}

	let { open = $bindable(), title, children, onclose }: Props = $props();

	function handleClose() {
		open = false;
		onclose?.();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') handleClose();
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) handleClose();
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
		style="animation: backdrop-enter 100ms ease-out"
		role="dialog"
		aria-modal="true"
		aria-label={title}
		onkeydown={handleKeydown}
		onclick={handleBackdropClick}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-full max-w-lg rounded-lg border border-border bg-surface-raised p-6 shadow-2xl"
			style="animation: dialog-enter 150ms ease-out"
			onclick={(e) => e.stopPropagation()}
		>
			<div class="mb-4 flex items-center justify-between">
				<h2 class="font-display text-xl">{title}</h2>
				<button
					onclick={handleClose}
					class="rounded-lg p-1 text-text-muted transition-colors hover:bg-surface-overlay hover:text-text"
				>
					&times;
				</button>
			</div>
			{@render children()}
		</div>
	</div>
{/if}
