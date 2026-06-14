<script lang="ts">
	import Select from '$lib/components/ui/Select.svelte';
	import DateInput from '$lib/components/ui/DateInput.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { ChevronDown, ChevronUp } from 'lucide-svelte';

	interface SaveEntry {
		frameNumber: string;
		aperture: string;
		shutterSpeed: string;
		lensId: string;
		date: string;
		location: string;
		notes: string;
	}

	interface Props {
		frameNumber: string;
		lensOptions: { value: string; label: string; disabled?: boolean }[];
		lensId: string;
		isFixedLens: boolean;
		fixedLensLabel: string;
		saving: boolean;
		error: string;
		onsave: (entry: SaveEntry) => void;
	}

	let {
		frameNumber,
		lensOptions,
		lensId: lensIdProp,
		isFixedLens,
		fixedLensLabel,
		saving,
		error,
		onsave
	}: Props = $props();

	// Internal state — lens persists across saves; per-shot fields reset
	let aperture = $state('');
	let shutterSpeed = $state('');
	let date = $state('');
	let location = $state('');
	let notes = $state('');
	let showMore = $state(false);

	// Local lens: seeded from prop and stays in sync when the prop changes (e.g. roll change).
	// Initialise to '' — the $effect runs immediately on mount and sets the real value.
	let localLensId = $state('');
	$effect(() => {
		localLensId = lensIdProp;
	});

	function handleSave() {
		if (!frameNumber.trim()) return;
		onsave({
			frameNumber,
			aperture,
			shutterSpeed,
			lensId: isFixedLens ? lensIdProp : localLensId,
			date,
			location,
			notes
		});
		// Clear per-shot fields on save (parent re-seeds frameNumber via the nextFrameNumber derived)
		aperture = '';
		shutterSpeed = '';
		notes = '';
		// Keep: date, location, lens (session defaults)
	}

	function handleKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'Enter' && frameNumber.trim() && !saving) {
			e.preventDefault();
			handleSave();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="rounded-lg border border-border bg-surface-raised p-3">
	<!-- Primary entry row -->
	<div class="flex flex-wrap items-end gap-2">
		<!-- Frame label (mono, read-only — parent drives this) -->
		<div class="flex flex-col gap-1">
			<span class="text-xs font-medium text-text-muted">Frame</span>
			<div
				class="flex h-[38px] min-w-[2.75rem] items-center justify-center rounded-lg border border-accent/40 bg-accent/10 px-2 font-mono text-sm font-medium text-accent"
			>
				{frameNumber || '—'}
			</div>
		</div>

		<!-- f/ Aperture input -->
		<div class="flex flex-col gap-1">
			<label for="qab-aperture" class="text-xs font-medium text-text-muted">f/</label>
			<input
				id="qab-aperture"
				data-field="aperture"
				type="text"
				bind:value={aperture}
				placeholder="5.6"
				class="h-[38px] w-20 rounded-lg border border-border bg-surface px-3 font-mono text-sm text-text placeholder-text-faint transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
			/>
		</div>

		<!-- Shutter speed input -->
		<div class="flex flex-col gap-1">
			<label for="qab-shutter" class="text-xs font-medium text-text-muted">Shutter</label>
			<input
				id="qab-shutter"
				type="text"
				bind:value={shutterSpeed}
				placeholder="1/250"
				class="h-[38px] w-24 rounded-lg border border-border bg-surface px-3 font-mono text-sm text-text placeholder-text-faint transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
			/>
		</div>

		<!-- Lens selector (or read-only fixed lens display) -->
		{#if isFixedLens}
			<div class="flex flex-col gap-1">
				<span class="text-xs font-medium text-text-muted">Lens</span>
				<div
					class="flex h-[38px] items-center rounded-lg border border-border bg-surface px-3 text-sm text-text-muted"
				>
					<span class="truncate max-w-[10rem]">{fixedLensLabel || 'Fixed lens'}</span>
				</div>
			</div>
		{:else}
			<div class="min-w-[10rem] flex-1">
				<Select label="Lens" options={lensOptions} bind:value={localLensId} />
			</div>
		{/if}

		<!-- Save & Next button -->
		<Button
			variant="primary"
			onclick={handleSave}
			disabled={saving || !frameNumber.trim()}
			title="Save frame and advance to next (⌘/Ctrl+Enter)"
		>
			{saving ? 'Saving…' : 'Save & Next'}
		</Button>

		<!-- More / less toggle -->
		<button
			onclick={() => (showMore = !showMore)}
			title={showMore ? 'Hide date, location, and notes' : 'Show date, location, and notes'}
			aria-label={showMore ? 'Hide extra fields' : 'Show extra fields'}
			aria-expanded={showMore}
			class="flex h-[38px] items-center gap-1 rounded-lg border border-border px-2.5 text-xs text-text-muted transition-colors hover:bg-surface-overlay hover:text-text"
		>
			<span class="hidden sm:inline">More</span>
			{#if showMore}
				<ChevronUp size={14} aria-hidden="true" />
			{:else}
				<ChevronDown size={14} aria-hidden="true" />
			{/if}
		</button>
	</div>

	<!-- Expanded fields: date / location / notes -->
	{#if showMore}
		<div class="mt-3 grid grid-cols-1 gap-3 border-t border-border-subtle pt-3 sm:grid-cols-3">
			<DateInput label="Date" bind:value={date} />
			<div class="flex flex-col gap-1">
				<label for="qab-location" class="text-xs font-medium text-text-muted">Location</label>
				<input
					id="qab-location"
					type="text"
					bind:value={location}
					placeholder="Where was this?"
					class="h-[38px] rounded-lg border border-border bg-surface px-3 text-sm text-text placeholder-text-faint transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
				/>
			</div>
			<Textarea label="Notes" bind:value={notes} rows={1} placeholder="Anything notable…" />
		</div>
	{/if}

	<!-- Inline error display -->
	{#if error}
		<p class="mt-2 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</p>
	{/if}
</div>
