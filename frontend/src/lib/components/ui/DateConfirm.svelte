<script lang="ts">
	// Small date-pick dialog used for both inline Timeline edits and the
	// confirm-on-transition prompt. Seeds an editable date (default today from the
	// caller), Confirm commits it, Cancel aborts. Inline edits also offer Clear
	// (commit null). There is no "Skip" — callers that want a blank date clear it
	// from the status timeline afterward.
	import Dialog from './Dialog.svelte';
	import DateInput from './DateInput.svelte';
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
		onconfirm: (date: string | null) => void;
		oncancel: () => void;
	}

	let {
		open = $bindable(),
		title,
		value = '',
		confirmLabel = 'Confirm',
		allowClear = false,
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
		<DateInput label="Date" bind:value={draft} />
		{#if !allowClear}
			<!-- Transition prompt (no Clear button): surface the otherwise-invisible
			     escape hatches — fuzzy dates are accepted by dateFieldError, and an
			     undated milestone can be edited or cleared later from the status timeline.
			     (kammerz-cb7) -->
			<p class="text-xs text-text-muted">
				A rough date is fine — enter just a year (2024) or year-month (2024-03). You can change or clear it later from
				the status timeline.
			</p>
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
