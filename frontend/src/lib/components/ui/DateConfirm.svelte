<script lang="ts">
	// Small, generic date-pick dialog. Seeds an editable date (default today from
	// the caller), Confirm commits it, Cancel aborts; `allowClear` adds a Clear
	// button (commit null). Where the date can be corrected later is a CALLER
	// concern — pass `hint` to name the right surface (e.g. the roll page's
	// activity board); the default stays feature-neutral.
	import Dialog from './Dialog.svelte';
	import Input from './Input.svelte';
	import Button from './Button.svelte';
	import { dateFieldError } from '$lib/utils/date';

	interface Props {
		open: boolean;
		title: string;
		/** Seed value; caller passes today for transition prompts, the current date for edits. */
		value?: string;
		confirmLabel?: string;
		/** Show a Clear button (commits null) — used for inline edits, not transitions. */
		allowClear?: boolean;
		/** Helper line shown when there's no Clear button; callers name their own edit surface. */
		hint?: string;
		onconfirm: (date: string | null) => void;
		oncancel: () => void;
	}

	let {
		open = $bindable(),
		title,
		value = '',
		confirmLabel = 'Confirm',
		allowClear = false,
		hint = 'Pick the date this happened — you can change or clear it later.',
		onconfirm,
		oncancel
	}: Props = $props();

	let draft = $state('');
	// Re-seed each time the dialog opens so a reused instance starts fresh.
	$effect(() => {
		if (open) draft = value;
	});

	// When clearing isn't allowed (the transition prompt, or setting a not-yet-set
	// date), an empty value can't Confirm — confirming nothing would be a backdoor
	// "Skip", which the design deliberately dropped. Enter a date or Cancel. When
	// `allowClear` is on, an empty Confirm is just an explicit clear (== the Clear
	// button), so it's allowed. Either way a malformed date can never be confirmed.
	const draftError = $derived(dateFieldError(draft));
	const canConfirm = $derived((allowClear || !!draft.trim()) && !draftError);

	function confirm() {
		onconfirm(draft.trim() ? draft.trim() : null);
	}
</script>

<Dialog bind:open {title} onclose={oncancel}>
	<div class="space-y-4">
		<Input type="date" label="Date" class="h-[38px]" bind:value={draft} />
		{#if !allowClear}
			<!-- No-Clear-button mode: surface the otherwise-invisible escape hatch. The
			     wording is caller-supplied via `hint` so a shared ui/ component never
			     hardcodes one page's section names. (kammerz-cb7) -->
			<p class="text-xs text-text-muted">{hint}</p>
		{/if}
		<div class="flex justify-end gap-2">
			{#if allowClear}
				<Button variant="ghost" onclick={() => onconfirm(null)}>Clear</Button>
			{/if}
			<Button variant="ghost" onclick={oncancel}>Cancel</Button>
			<Button variant="primary" disabled={!canConfirm} onclick={confirm}>{confirmLabel}</Button>
		</div>
	</div>
</Dialog>
