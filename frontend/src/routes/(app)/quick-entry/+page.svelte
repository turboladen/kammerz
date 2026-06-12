<script lang="ts">
	import { untrack } from 'svelte';
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import DateInput from '$lib/components/ui/DateInput.svelte';
	import { listRolls, updateRoll } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import { listLenses } from '$lib/api/lenses';
	import { listShotsForRoll, createShot, suggestNextFrame } from '$lib/api/shots';
	import { buildLensOptions, lensDisplayName } from '$lib/utils/lens';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import { getStatusLabel } from '$lib/utils/status';
	import { todayLocal, dateFieldError } from '$lib/utils/date';
	import type { RollWithDetails, Camera, Lens, LensMount, Shot } from '$lib/types';

	let rolls: RollWithDetails[] = $state([]);
	let cameras: Camera[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let lensMounts: LensMount[] = $state([]);
	let selectedRollId = $state('');
	let shots: Shot[] = $state([]);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');
	let successMessage = $state('');
	let sessionCount = $state(0);
	let lastSavedFrame = $state('');

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
		const options: { value: string; label: string; disabled?: boolean }[] = [{ value: '', label: 'Select a roll...' }];
		for (const r of active) {
			const filmInfo = r.film_stock_brand ? ` — ${r.film_stock_brand} ${r.film_stock_name}` : '';
			options.push({ value: String(r.id), label: `${r.roll_id}${filmInfo} (${getStatusLabel(r.status)})` });
		}
		if (other.length > 0 && active.length > 0) {
			options.push({ value: '__divider__', label: '── Other rolls ──', disabled: true });
		}
		for (const r of other) {
			const filmInfo = r.film_stock_brand ? ` — ${r.film_stock_brand} ${r.film_stock_name}` : '';
			options.push({ value: String(r.id), label: `${r.roll_id}${filmInfo} (${getStatusLabel(r.status)})` });
		}
		return options;
	});

	const selectedRoll = $derived(rolls.find((r) => String(r.id) === selectedRollId));

	const selectedCamera = $derived(
		selectedRoll?.camera_id ? (cameras.find((c) => c.id === selectedRoll.camera_id) ?? null) : null
	);

	const isFixedLens = $derived(
		selectedCamera ? lensMounts.some((m) => m.id === selectedCamera.lens_mount_id && m.name === 'Fixed Lens') : false
	);
	const fixedLens = $derived(
		isFixedLens && selectedCamera?.default_lens_id
			? (allLenses.find((l) => l.id === selectedCamera.default_lens_id) ?? null)
			: null
	);

	const lensOptions = $derived(buildLensOptions(allLenses, selectedCamera, 'No lens selected', lensMounts));

	// Frame progress for selected roll
	const frameInfo = $derived.by(() => {
		if (!selectedRoll) return null;
		const total = selectedRoll.frame_count;
		if (!total) return { current: shots.length, total: null };
		return { current: shots.length, total };
	});

	// Roll-full nudge state
	let rollFullDismissed = $state(false);

	// Finish date shown in the roll-complete nudge — written as date_finished when the
	// roll is marked Shot, so the Timeline's "Finished shooting" milestone isn't left
	// blank (kammerz-fis). Seeded per-roll in loadRollData: an already-set date_finished,
	// else today.
	let finishDate = $state(todayLocal());
	const finishDateError = $derived(dateFieldError(finishDate));

	const showRollFullNudge = $derived(
		selectedRoll?.status === 'shooting' &&
			frameInfo !== null &&
			frameInfo.total !== null &&
			shots.length >= frameInfo.total &&
			!rollFullDismissed
	);

	async function markRollShot() {
		if (!selectedRoll || !finishDate.trim() || finishDateError) return;
		try {
			await updateRoll(selectedRoll.id, { status: 'shot', date_finished: finishDate });
			rolls = await listRolls();
			rollFullDismissed = true;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function loadInitial() {
		try {
			const [r, cams, lenses, mounts] = await Promise.all([listRolls(), listCameras(), listLenses(), listLensMounts()]);
			rolls = r;
			cameras = cams;
			allLenses = lenses;
			lensMounts = mounts;

			// Pre-select roll from query param (e.g., /quick-entry?roll=42)
			const rollParam = page.url.searchParams.get('roll');
			if (rollParam && r.some((roll) => String(roll.id) === rollParam)) {
				selectedRollId = rollParam;
			}
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	async function loadRollData(rollId: number) {
		error = '';
		try {
			shots = await listShotsForRoll(rollId);

			// Seed the finish-date nudge: an already-recorded date_finished wins, else today.
			// (Mirrors the roll-detail nudge; quick-entry shots carry no per-shot date.)
			const roll = rolls.find((r) => r.id === rollId);
			finishDate = roll?.date_finished ?? todayLocal();

			// Suggest next frame
			try {
				frameNumber = await suggestNextFrame(rollId);
			} catch {
				frameNumber = '';
			}
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
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

			sessionCount++;
			lastSavedFrame = frameNumber.trim();
			successMessage = `Frame ${frameNumber} saved`;
			setTimeout(() => (successMessage = ''), 2000);

			// Auto-advance: reload shots + rolls (status may have changed), clear fields, keep lens, suggest next frame
			[shots, rolls] = await Promise.all([listShotsForRoll(Number(selectedRollId)), listRolls()]);
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

	// When roll changes, reload data and auto-populate lens.
	// Only selectedRollId is tracked — rolls/cameras reads are untracked so the
	// rolls refetch in handleSave doesn't re-run this effect and clobber a
	// manually selected lens or duplicate loadRollData fetches (kammerz-cpp).
	$effect(() => {
		if (selectedRollId) {
			rollFullDismissed = false;
			loadRollData(Number(selectedRollId));
			untrack(() => {
				const roll = rolls.find((r) => String(r.id) === selectedRollId);
				const camera = roll?.camera_id ? (cameras.find((c) => c.id === roll.camera_id) ?? null) : null;
				if (roll?.lens_id) {
					selectedLensId = String(roll.lens_id);
				} else if (camera?.default_lens_id) {
					selectedLensId = String(camera.default_lens_id);
				} else {
					// No applicable default — clear so a lens from a previous
					// roll (possibly a different camera system) can't carry over
					selectedLensId = '';
				}
			});
		} else {
			shots = [];
			frameNumber = '';
			selectedLensId = '';
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
			<FadeIn>
				<div
					class="mb-5 flex flex-wrap items-center gap-3 rounded-lg border border-border bg-surface-raised px-4 py-2.5 text-sm text-text-muted"
				>
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
			</FadeIn>

			{#if showRollFullNudge}
				<FadeIn delay={50}>
					<div
						class="mb-5 flex flex-wrap items-end justify-between gap-3 rounded-lg border border-accent/30 bg-accent/10 px-4 py-3"
					>
						<div class="flex flex-col gap-2">
							<div>
								<p class="text-sm font-medium text-accent">Roll complete</p>
								<p class="text-xs text-accent/70">
									All {frameInfo?.total} frames shot. When did you finish it?
								</p>
							</div>
							<div class="w-44">
								<DateInput label="Finished shooting" bind:value={finishDate} />
							</div>
						</div>
						<div class="flex items-center gap-2">
							<Button
								size="sm"
								variant="primary"
								disabled={!finishDate.trim() || !!finishDateError}
								onclick={markRollShot}>Mark as Shot</Button
							>
							<button
								onclick={() => {
									rollFullDismissed = true;
								}}
								class="text-accent/60 hover:text-accent transition-colors text-lg leading-none px-1"
								aria-label="Dismiss">&times;</button
							>
						</div>
					</div>
				</FadeIn>
			{/if}

			<!-- Entry Form -->
			<FadeIn delay={50}>
				<div class="mb-6 rounded-lg border border-border bg-surface-raised p-5">
					<div class="grid grid-cols-2 gap-3 md:grid-cols-4">
						<Input label="Frame" bind:value={frameNumber} placeholder="1" required />
						<Input label="f/" bind:value={aperture} placeholder="5.6" data-field="aperture" />
						<Input label="Speed" bind:value={shutterSpeed} placeholder="1/125" />
						{#if isFixedLens && fixedLens}
							<div>
								<span class="mb-1.5 block text-xs font-medium text-text-muted">Lens</span>
								<div class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text-muted truncate">
									{lensDisplayName(fixedLens)} <span class="text-text-faint">(fixed)</span>
								</div>
							</div>
						{:else}
							<Select label="Lens" bind:value={selectedLensId} options={lensOptions} />
						{/if}
					</div>
					<div class="mt-3">
						<Textarea label="Notes" bind:value={notes} placeholder="Optional notes..." />
					</div>

					{#if error}
						<div class="mt-3 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
					{/if}

					{#if successMessage}
						{#key successMessage}
							<div class="mt-3 animate-success-flash rounded-lg bg-green-500/15 px-3 py-2 text-sm text-green-400">
								{successMessage}
							</div>
						{/key}
					{/if}

					<div class="mt-4 flex items-center justify-between">
						<div class="flex items-center gap-3">
							<span class="text-xs text-text-faint">
								{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}+Enter to save
							</span>
							{#if sessionCount > 0}
								<span class="font-mono text-xs text-text-faint">{sessionCount} this session</span>
							{/if}
						</div>
						<Button variant="primary" onclick={handleSave} disabled={saving || !frameNumber.trim()}>
							{saving ? 'Saving...' : 'Save & Next →'}
						</Button>
					</div>
				</div>
			</FadeIn>

			<!-- Recent Shots -->
			{#if shots.length > 0}
				<div>
					<h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-text-faint">Previous Shots</h2>
					<div class="space-y-1">
						{#each [...shots].reverse().slice(0, 10) as shot, i}
							<div
								class="flex items-center gap-3 rounded px-3 py-1.5 text-sm text-text-muted {i === 0 &&
								shot.frame_number === lastSavedFrame
									? 'animate-fade-in-up'
									: ''}"
							>
								<span
									class="inline-flex h-5 min-w-5 items-center justify-center rounded bg-accent/10 px-1 font-mono text-xs text-accent"
								>
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
