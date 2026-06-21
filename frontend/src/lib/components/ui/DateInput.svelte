<script lang="ts">
	import { Calendar } from 'lucide-svelte';
	import { dateFieldError } from '$lib/utils/date';

	interface Props {
		label?: string;
		hint?: string;
		value?: string;
	}

	let { label, hint, value = $bindable('') }: Props = $props();

	// Still-typing patterns — don't show errors for partial input in progress
	const typingPattern = /^(\d{0,4}|\d{4}-\d{0,2}|\d{4}-\d{2}-\d{0,2})$/;

	const validationError = $derived.by(() => {
		// Trim first so the live-error display matches dateFieldError (the documented
		// source of truth, which trims) and the Save/Confirm gates — otherwise a
		// surrounding space shows an error here while the gate stays satisfied.
		const v = value.trim();
		if (!v) return '';
		// Complete pattern → defer to the shared validator (same rule the
		// Save/Confirm gates and the backend enforce).
		if (/^(\d{4})(-\d{2}(-\d{2})?)?$/.test(v)) return dateFieldError(v);
		// If they're still typing, don't nag yet
		if (typingPattern.test(v)) return '';
		return 'Use YYYY, YYYY-MM, or YYYY-MM-DD';
	});

	let hiddenDateInput: HTMLInputElement | undefined = $state();

	function handlePickerChange(e: Event) {
		const target = e.target as HTMLInputElement;
		if (target.value) {
			value = target.value;
		}
	}
</script>

<label class="flex flex-col gap-1.5">
	{#if label}
		<span class="text-xs font-medium text-text-muted">{label}</span>
	{/if}
	<div class="relative">
		<input
			type="text"
			bind:value
			placeholder="YYYY-MM-DD"
			class="w-full rounded-lg border border-border bg-surface px-3 py-2 pr-9 font-mono text-sm text-text placeholder-text-faint
				transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50
				{validationError ? 'border-red-500/60' : ''}"
		/>
		<!-- Calendar icon (visual only — the real click target is the overlay input) -->
		<div class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-text-faint">
			<Calendar size={14} strokeWidth={1.5} />
		</div>
		<!-- Native date input overlaid on the calendar icon — gives WebKit a real element to anchor/dismiss the picker -->
		<input
			bind:this={hiddenDateInput}
			type="date"
			onchange={handlePickerChange}
			class="absolute right-1 top-1/2 -translate-y-1/2 cursor-pointer opacity-0"
			style="width: 24px; height: 24px;"
			tabindex={-1}
			title="Pick from calendar"
		/>
	</div>
	{#if validationError}
		<span class="text-xs text-red-400">{validationError}</span>
	{:else if hint}
		<span class="text-xs text-text-faint">{hint}</span>
	{/if}
</label>
