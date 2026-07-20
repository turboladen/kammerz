<script lang="ts">
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import RollRow from '$lib/components/rolls/RollRow.svelte';
	import QuickAddBar from '$lib/components/rolls/QuickAddBar.svelte';
	import FrameStrip from '$lib/components/rolls/FrameStrip.svelte';
	import { Film } from 'lucide-svelte';
	import { listRolls, updateRoll } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import { listLenses } from '$lib/api/lenses';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import { listShotsForRoll } from '$lib/api/shots';
	import { logShot } from '$lib/utils/shot-entry';
	import { buildLensOptions, lensDisplayName } from '$lib/utils/lens';
	import { buildFrameCells, nextExtraFrameNumber } from '$lib/utils/frames';
	import { todayLocal, dateFieldError } from '$lib/utils/date';
	import type { RollWithDetails, Camera, Lens, LensMount, Shot } from '$lib/types';

	let rolls: RollWithDetails[] = $state([]);
	let cameras: Camera[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let lensMounts: LensMount[] = $state([]);
	let selectedRollId = $state('');
	let shots: Shot[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let sessionCount = $state(0);

	// QuickAddBar save state
	let quickSaving = $state(false);
	let quickError = $state('');

	// When set (via a FrameStrip click on an open slot or the "+" extra button), the
	// QuickAddBar logs THIS frame instead of the sequential next one. Cleared after a
	// successful save so logging returns to sequential.
	let jumpFrame = $state('');

	// Active rolls only — the ones you'd realistically log frames against: those
	// whose development activity hasn't started (loaded / shooting / finished-shooting).
	// Once development or any tail activity begins, quick logging is no longer offered
	// (ADR-0013) — including a roll with a tail start date but no dev record.
	const activeRolls = $derived(
		rolls.filter((r) => r.activities.find((a) => a.kind === 'development')?.state === 'not_started')
	);

	const selectedRoll = $derived(rolls.find((r) => String(r.id) === selectedRollId) ?? null);

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

	// Smart lens default: fixed > roll default > camera default (QuickAddBar seeds from this).
	const quickDefaultLensId = $derived.by(() => {
		if (fixedLens) return String(fixedLens.id);
		if (selectedRoll?.lens_id) return String(selectedRoll.lens_id);
		if (selectedCamera?.default_lens_id) return String(selectedCamera.default_lens_id);
		return '';
	});

	const frameCells = $derived(selectedRoll ? buildFrameCells(shots, selectedRoll.frame_count) : []);
	const nextFrameNumber = $derived(frameCells.find((c) => c.isNext)?.frameNumber ?? '');
	const frameToLog = $derived(jumpFrame || nextFrameNumber);

	// Frame progress for the roll-full nudge.
	const frameInfo = $derived.by(() => {
		if (!selectedRoll) return null;
		const total = selectedRoll.frame_count;
		return { current: shots.length, total: total ?? null };
	});

	// Roll-full nudge state
	let rollFullDismissed = $state(false);
	let finishDate = $state(todayLocal());
	const finishDateError = $derived(dateFieldError(finishDate));

	const showRollFullNudge = $derived(
		selectedRoll?.activities.find((a) => a.kind === 'shooting')?.state === 'in_progress' &&
			frameInfo !== null &&
			frameInfo.total !== null &&
			shots.length >= frameInfo.total &&
			!rollFullDismissed
	);

	async function markRollShot() {
		if (!selectedRoll || !finishDate.trim() || finishDateError) return;
		try {
			await updateRoll(selectedRoll.id, { date_finished: finishDate });
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
			const roll = rolls.find((r) => r.id === rollId);
			finishDate = roll?.date_finished ?? todayLocal();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleQuickAdd(entry: {
		frameNumber: string;
		aperture: string;
		shutterSpeed: string;
		lensId: string;
		date: string;
		time: string;
		location: string;
		notes: string;
	}): Promise<boolean> {
		if (!selectedRoll || !entry.frameNumber.trim()) return false;
		quickError = '';
		quickSaving = true;
		try {
			await logShot({ rollId: selectedRoll.id, ...entry });
			sessionCount++;
			jumpFrame = '';
			[shots, rolls] = await Promise.all([listShotsForRoll(selectedRoll.id), listRolls()]);
			return true;
		} catch (err) {
			quickError = err instanceof Error ? err.message : String(err);
			return false;
		} finally {
			quickSaving = false;
		}
	}

	// FrameStrip: open slot → target it next; filled frame → no-op (edit on roll page).
	function handleFrameSelect(frameNumber: string, shot: Shot | null) {
		if (shot === null) jumpFrame = frameNumber;
	}

	function handleAddExtra() {
		if (selectedRoll) jumpFrame = nextExtraFrameNumber(shots, selectedRoll.frame_count);
	}

	function changeRoll() {
		selectedRollId = '';
	}

	// Load a roll's shots when the selection changes. Only selectedRollId is tracked;
	// loadRollData's reactive reads happen after its first await, so this never loops.
	$effect(() => {
		if (selectedRollId) {
			rollFullDismissed = false;
			jumpFrame = '';
			loadRollData(Number(selectedRollId));
		} else {
			shots = [];
			quickError = '';
			error = '';
		}
	});

	$effect(() => {
		loadInitial();
	});
</script>

<PageHeader title="Quick Entry" description="Rapid shot logging — one frame at a time" />

<div class="p-6">
	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else}
		{#if error}
			<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}

		{#if !selectedRoll}
			<!-- Roll picker: visual list of active rolls -->
			<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
				Active rolls
				<div class="flex-1 border-b border-border-subtle"></div>
			</h2>
			{#if activeRolls.length === 0}
				<EmptyState title="No active rolls" message="Load a roll to start logging shots.">
					{#snippet icon()}<Film size={24} strokeWidth={1.5} />{/snippet}
					<Button variant="primary" href="/rolls/new">+ New Roll</Button>
				</EmptyState>
			{:else}
				<div class="grid gap-1.5">
					{#each activeRolls as roll, i (roll.id)}
						<FadeIn delay={Math.min(i, 10) * 30}>
							<RollRow {roll} onclick={() => (selectedRollId = String(roll.id))} />
						</FadeIn>
					{/each}
				</div>
			{/if}
		{:else}
			<!-- Collapsed selected roll + Change -->
			<div class="mb-5">
				<RollRow roll={selectedRoll} selected>
					{#snippet trailing()}
						<Button size="sm" variant="ghost" onclick={changeRoll}>Change</Button>
					{/snippet}
				</RollRow>
			</div>

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
								<Input type="date" label="Finished shooting" class="h-[38px]" bind:value={finishDate} />
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
								class="px-1 text-lg leading-none text-accent/60 transition-colors hover:text-accent"
								aria-label="Dismiss">&times;</button
							>
						</div>
					</div>
				</FadeIn>
			{/if}

			<!-- Entry bar + film strip -->
			<FadeIn delay={50}>
				<div class="space-y-2">
					<QuickAddBar
						frameNumber={frameToLog}
						{lensOptions}
						lensId={quickDefaultLensId}
						{isFixedLens}
						fixedLensLabel={fixedLens ? lensDisplayName(fixedLens) : ''}
						saving={quickSaving}
						error={quickError}
						onsave={handleQuickAdd}
					/>
					<!-- filledClickable={false}: clicking a filled frame is a no-op on this page
				     (viewing/editing lives on the roll page), so the label must not
				     advertise a click action that doesn't exist. -->
					<FrameStrip
						frames={frameCells}
						onselect={handleFrameSelect}
						onaddextra={handleAddExtra}
						filledClickable={false}
					/>
				</div>
			</FadeIn>

			{#if sessionCount > 0}
				<p class="mt-3 font-mono text-xs text-text-faint">{sessionCount} this session</p>
			{/if}
		{/if}
	{/if}
</div>
