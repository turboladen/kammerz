<script lang="ts">
	import { Clock } from 'lucide-svelte';
	import { parseTime, timeFieldError } from '$lib/utils/time';

	interface Props {
		label?: string;
		hint?: string;
		value?: string;
	}

	let { label, hint, value = $bindable('') }: Props = $props();

	// The app is 24-hour (ADR-0005). Native `<input type="time">` renders 12h in
	// the browser locale regardless of the OS setting, so time is a plain 24h
	// `HH:MM` text field — no native picker overlay (that would reintroduce the
	// 12h widget). Dates don't have this problem — `<input type="date">` has no
	// locale-dependent 12/24h ambiguity — so date entry uses the native picker
	// directly (see `Input type="date"` call sites) instead of a custom field.
	const validationError = $derived(timeFieldError(value ?? ''));

	// Canonicalize to zero-padded `HH:MM` on blur so the stored value matches the
	// existing data and the backend's strict `validate_time` (e.g. `1430` → `14:30`,
	// `9:05` → `09:05`). Clicking a Save button blurs the field first, so the save
	// payload sees the normalized value. Invalid/blank input is left untouched (the
	// inline error / optional-field handling covers it).
	function normalizeOnBlur() {
		const parsed = parseTime(value ?? '');
		// parseTime → '' (blank), null (invalid), or canonical 'HH:MM'. Normalize on
		// blank (resets stray whitespace to '') and on valid; leave invalid untouched
		// so the inline error stays visible.
		if (parsed !== null) value = parsed;
	}
</script>

<label class="flex flex-col gap-1.5">
	{#if label}
		<span class="text-xs font-medium text-text-muted">{label}</span>
	{/if}
	<div class="relative">
		<input
			type="text"
			inputmode="numeric"
			bind:value
			onblur={normalizeOnBlur}
			placeholder="HH:MM"
			class="w-full rounded-lg border border-border bg-surface px-3 py-2 pr-9 font-mono text-sm text-text placeholder-text-faint
				transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50
				{validationError ? 'border-red-500/60' : ''}"
		/>
		<div class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-text-faint">
			<Clock size={14} strokeWidth={1.5} />
		</div>
	</div>
	{#if validationError}
		<span class="text-xs text-red-400">{validationError}</span>
	{:else if hint}
		<span class="text-xs text-text-faint">{hint}</span>
	{/if}
</label>
