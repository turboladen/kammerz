<script lang="ts">
	import Button from './Button.svelte';

	interface Props {
		open: boolean;
		title?: string;
		message: string;
		confirmLabel?: string;
		cancelLabel?: string;
		variant?: 'danger' | 'primary';
		onconfirm: () => void;
		oncancel: () => void;
	}

	let {
		open = $bindable(),
		title = 'Are you sure?',
		message,
		confirmLabel = 'Delete',
		cancelLabel = 'Cancel',
		variant = 'danger',
		onconfirm,
		oncancel
	}: Props = $props();

	function handleCancel() {
		open = false;
		oncancel();
	}

	function handleConfirm() {
		open = false;
		onconfirm();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') handleCancel();
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) handleCancel();
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
		style="animation: backdrop-enter 100ms ease-out"
		role="alertdialog"
		aria-modal="true"
		aria-label={title}
		onkeydown={handleKeydown}
		onclick={handleBackdropClick}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-full max-w-sm rounded-lg border border-border bg-surface-raised p-6 shadow-2xl"
			style="animation: dialog-enter 150ms ease-out"
			onclick={(e) => e.stopPropagation()}
		>
			<h2 class="font-display text-xl">{title}</h2>
			<p class="mt-2 text-sm text-text-muted">{message}</p>
			<div class="mt-5 flex justify-end gap-2">
				<Button variant="ghost" onclick={handleCancel}>{cancelLabel}</Button>
				<Button variant={variant} onclick={handleConfirm}>{confirmLabel}</Button>
			</div>
		</div>
	</div>
{/if}
