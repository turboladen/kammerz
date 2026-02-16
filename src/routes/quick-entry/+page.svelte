<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import { listRolls } from '$lib/api/rolls';
	import { getLensesForCamera } from '$lib/api/cameras';
	import { listLenses } from '$lib/api/lenses';
	import { listShotsForRoll, createShot, suggestNextFrame } from '$lib/api/shots';
	import { lensDisplayName } from '$lib/utils/lens';
	import type { RollWithDetails, Lens, Shot } from '$lib/types';

	let rolls: RollWithDetails[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let selectedRollId = $state('');
	let shots: Shot[] = $state([]);
	let cameraLensIds: number[] = $state([]);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');
	let successMessage = $state('');

	// Form fields
	let frameNumber = $state('');
	let aperture = $state('');
	let shutterSpeed = $state('');
	let selectedLensId = $state('');
	let notes = $state('');

	// Active rolls (loaded/shooting/shot) first, then others
	const activeStatuses = ['loaded', 'shooting', 'shot'];
	const rollOptions = $derived.by(() => {
		const active = rolls.filter((r) => activeStatuses.includes(r.status));
		const other = rolls.filter((r) => !activeStatuses.includes(r.status));
		const options: { value: string; label: string; disabled?: boolean }[] = [
			{ value: '', label: 'Select a roll...' }
		];
		for (const r of active) {
			const filmInfo = r.film_stock_brand ? ` — ${r.film_stock_brand} ${r.film_stock_name}` : '';
			options.push({ value: String(r.id), label: `${r.roll_id}${filmInfo} (${r.status})` });
		}
		if (other.length > 0 && active.length > 0) {
			options.push({ value: '', label: '── Other rolls ──', disabled: true });
		}
		for (const r of other) {
			const filmInfo = r.film_stock_brand ? ` — ${r.film_stock_brand} ${r.film_stock_name}` : '';
			options.push({ value: String(r.id), label: `${r.roll_id}${filmInfo} (${r.status})` });
		}
		return options;
	});

	const selectedRoll = $derived(rolls.find((r) => String(r.id) === selectedRollId));

	// Lens options for the selected camera
	const lensOptions = $derived.by(() => {
		const owned = allLenses.filter((l) => !l.date_sold);
		// Camera-linked first, then others
		const linked = owned.filter((l) => cameraLensIds.includes(l.id));
		const other = owned.filter((l) => !cameraLensIds.includes(l.id));
		const options: { value: string; label: string; disabled?: boolean }[] = [
			{ value: '', label: 'No lens selected' }
		];
		if (linked.length > 0) {
			for (const l of linked) {
				options.push({ value: String(l.id), label: lensDisplayName(l) });
			}
			if (other.length > 0) {
				options.push({ value: '', label: '── Other lenses ──', disabled: true });
			}
		}
		for (const l of other) {
			options.push({ value: String(l.id), label: lensDisplayName(l) });
		}
		return options;
	});

	// Frame progress for selected roll
	const frameInfo = $derived.by(() => {
		if (!selectedRoll) return null;
		const total = selectedRoll.frame_count;
		if (!total) return { current: shots.length, total: null };
		return { current: shots.length, total };
	});

	async function loadInitial() {
		try {
			const [r, lenses] = await Promise.all([listRolls(), listLenses()]);
			rolls = r;
			allLenses = lenses;
		} finally {
			loading = false;
		}
	}

	async function loadRollData(rollId: number) {
		const [s, roll] = await Promise.all([
			listShotsForRoll(rollId),
			Promise.resolve(rolls.find((r) => r.id === rollId))
		]);
		shots = s;

		// Load camera-lens associations
		if (roll?.camera_id) {
			cameraLensIds = await getLensesForCamera(roll.camera_id);
		} else {
			cameraLensIds = [];
		}

		// Suggest next frame
		try {
			frameNumber = await suggestNextFrame(rollId);
		} catch {
			frameNumber = '';
		}
	}

	async function handleSave() {
		if (!selectedRollId || !frameNumber.trim()) {
			error = 'Please select a roll and enter a frame number.';
			return;
		}
		error = '';
		saving = true;
		try {
			const lensIds = selectedLensId ? [Number(selectedLensId)] : [];
			await createShot({
				roll_id: Number(selectedRollId),
				frame_number: frameNumber.trim(),
				aperture: aperture || null,
				shutter_speed: shutterSpeed || null,
				date: null,
				date_fuzzy: null,
				location: null,
				gps_lat: null,
				gps_lon: null,
				notes: notes || null,
				lens_ids: lensIds
			});

			successMessage = `Frame ${frameNumber} saved`;
			setTimeout(() => (successMessage = ''), 2000);

			// Auto-advance: reload shots, clear fields, keep lens, suggest next frame
			shots = await listShotsForRoll(Number(selectedRollId));
			aperture = '';
			shutterSpeed = '';
			notes = '';
			// Keep selectedLensId — photographer usually uses the same lens per roll

			try {
				frameNumber = await suggestNextFrame(Number(selectedRollId));
			} catch {
				frameNumber = '';
			}

			// Focus the aperture field after save
			setTimeout(() => {
				const apertureInput = document.querySelector<HTMLInputElement>('[data-field="aperture"]');
				apertureInput?.focus();
			}, 50);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
			e.preventDefault();
			handleSave();
		}
	}

	// When roll changes, reload data
	$effect(() => {
		if (selectedRollId) {
			loadRollData(Number(selectedRollId));
		} else {
			shots = [];
			cameraLensIds = [];
			frameNumber = '';
		}
	});

	$effect(() => {
		loadInitial();
	});
</script>

<svelte:window onkeydown={handleKeydown} />

<PageHeader title="Quick Entry" description="Rapid shot logging — one frame at a time" />

<div class="p-6">
	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else}
		<!-- Roll Selector -->
		<div class="mb-5">
			<Select label="Roll" bind:value={selectedRollId} options={rollOptions} />
		</div>

		{#if selectedRoll}
			<!-- Roll Info Bar -->
			<div class="mb-5 flex flex-wrap items-center gap-3 rounded-lg border border-border bg-surface-raised px-4 py-2.5 text-sm text-text-muted">
				{#if selectedRoll.camera_brand}
					<span>{selectedRoll.camera_brand} {selectedRoll.camera_model}</span>
					<span class="text-text-faint">&middot;</span>
				{/if}
				{#if selectedRoll.film_stock_brand}
					<span>{selectedRoll.film_stock_brand} {selectedRoll.film_stock_name}</span>
				{/if}
				{#if selectedRoll.film_stock_iso}
					<span class="text-text-faint">ISO {selectedRoll.film_stock_iso}</span>
				{/if}
				{#if frameInfo}
					<span class="text-text-faint">&middot;</span>
					<span class="text-text-faint">
						{frameInfo.current}{frameInfo.total ? `/${frameInfo.total}` : ''} frames
					</span>
					{#if frameInfo.total}
						<div class="h-1.5 w-20 overflow-hidden rounded-full bg-surface-overlay">
							<div
								class="h-full rounded-full bg-accent transition-all duration-300"
								style="width: {Math.min((frameInfo.current / frameInfo.total) * 100, 100)}%"
							></div>
						</div>
					{/if}
				{/if}
			</div>

			<!-- Entry Form -->
			<div class="mb-6 rounded-lg border border-border bg-surface-raised p-5">
				<div class="grid grid-cols-4 gap-3">
					<Input label="Frame" bind:value={frameNumber} placeholder="1" required />
					<Input label="f/" bind:value={aperture} placeholder="5.6" data-field="aperture" />
					<Input label="Speed" bind:value={shutterSpeed} placeholder="1/125" />
					<Select label="Lens" bind:value={selectedLensId} options={lensOptions} />
				</div>
				<div class="mt-3">
					<Textarea label="Notes" bind:value={notes} placeholder="Optional notes..." />
				</div>

				{#if error}
					<div class="mt-3 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
				{/if}

				{#if successMessage}
					<div class="mt-3 rounded-lg bg-green-500/15 px-3 py-2 text-sm text-green-400">{successMessage}</div>
				{/if}

				<div class="mt-4 flex items-center justify-between">
					<span class="text-xs text-text-faint">
						{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}+Enter to save
					</span>
					<Button variant="primary" onclick={handleSave} disabled={saving || !frameNumber.trim()}>
						{saving ? 'Saving...' : 'Save & Next →'}
					</Button>
				</div>
			</div>

			<!-- Recent Shots -->
			{#if shots.length > 0}
				<div>
					<h2 class="mb-2 text-sm font-semibold text-text-muted">Previous shots</h2>
					<div class="space-y-1">
						{#each [...shots].reverse().slice(0, 10) as shot}
							<div class="flex items-center gap-3 rounded px-3 py-1.5 text-sm text-text-muted">
								<span class="inline-flex h-5 min-w-5 items-center justify-center rounded bg-accent/10 px-1 font-mono text-xs text-accent">
									{shot.frame_number}
								</span>
								{#if shot.aperture}
									<span>f/{shot.aperture}</span>
								{/if}
								{#if shot.shutter_speed}
									<span>{shot.shutter_speed}</span>
								{/if}
								{#if shot.notes}
									<span class="text-text-faint">{shot.notes}</span>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}
		{:else}
			<p class="text-sm text-text-faint">Select a roll to start logging shots.</p>
		{/if}
	{/if}
</div>
