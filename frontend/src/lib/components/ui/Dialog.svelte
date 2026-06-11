<script lang="ts">
	import { dialogFocus } from '$lib/utils/focus';

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

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) handleClose();
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
		style="animation: backdrop-enter 100ms ease-out"
		role="dialog"
		aria-modal="true"
		aria-label={title}
		onclick={handleBackdropClick}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			class="flex w-full max-w-lg flex-col rounded-lg border border-border bg-surface-raised shadow-2xl max-h-[85vh] focus:outline-none"
			style="animation: dialog-enter 150ms ease-out"
			tabindex="-1"
			use:dialogFocus={{ onEscape: handleClose }}
			onclick={(e) => e.stopPropagation()}
		>
			<div class="flex items-center justify-between px-6 pt-6 pb-4 shrink-0">
				<h2 class="font-display text-xl">{title}</h2>
				<button
					onclick={handleClose}
					aria-label="Close"
					class="rounded-lg p-1 text-text-muted transition-colors hover:bg-surface-overlay hover:text-text"
				>
					&times;
				</button>
			</div>
			<div class="overflow-y-auto px-6 pb-6">
				{@render children()}
			</div>
		</div>
	</div>
{/if}
