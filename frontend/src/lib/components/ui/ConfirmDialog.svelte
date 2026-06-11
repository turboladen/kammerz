<script lang="ts">
	import Button from './Button.svelte';
	import { AlertTriangle, Trash2 } from 'lucide-svelte';
	import { dialogFocus } from '$lib/utils/focus';

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

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) handleCancel();
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
		style="animation: backdrop-enter 100ms ease-out"
		role="alertdialog"
		aria-modal="true"
		aria-label={title}
		onclick={handleBackdropClick}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			class="w-full max-w-sm rounded-lg border border-border bg-surface-raised p-6 shadow-2xl focus:outline-none"
			style="animation: dialog-enter 150ms ease-out"
			tabindex="-1"
			use:dialogFocus={{ onEscape: handleCancel }}
			onclick={(e) => e.stopPropagation()}
		>
			<div class="flex items-center gap-3">
				{#if variant === 'danger'}
					<span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-danger/15 text-danger-fg">
						<AlertTriangle size={18} strokeWidth={2} aria-hidden="true" />
					</span>
				{/if}
				<h2 class="font-display text-xl">{title}</h2>
			</div>
			<p class="mt-2 text-sm text-text-muted">{message}</p>
			<div class="mt-5 flex justify-end gap-2">
				<Button variant="ghost" onclick={handleCancel}>{cancelLabel}</Button>
				<Button variant={variant} onclick={handleConfirm}>
					{#if variant === 'danger'}<Trash2 size={16} strokeWidth={2} aria-hidden="true" />{/if}
					{confirmLabel}
				</Button>
			</div>
		</div>
	</div>
{/if}
