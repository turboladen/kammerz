<script lang="ts">
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import DateInput from '$lib/components/ui/DateInput.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import { ChevronUp, ChevronDown, X } from 'lucide-svelte';
	import {
		createLabDev,
		updateLabDev,
		deleteLabDev,
		createSelfDev,
		updateSelfDev,
		deleteSelfDev
	} from '$lib/api/development';
	import { secondsToMmSs, mmSsToSeconds } from '$lib/utils/duration';
	import type { Lab, DevelopmentLab, DevelopmentSelf, DevStage } from '$lib/types';

	let {
		rollId,
		labs,
		labDev = $bindable(),
		selfDev = $bindable(),
		devStages = $bindable(),
		autoPrompt = $bindable(null),
		defaultDate = '',
		onchange
	}: {
		rollId: number;
		labs: Lab[];
		labDev: DevelopmentLab | null;
		selfDev: DevelopmentSelf | null;
		devStages: DevStage[];
		autoPrompt: 'lab' | 'self' | null;
		defaultDate?: string;
		onchange: () => Promise<void>;
	} = $props();

	// Dialog visibility
	let showLabDevDialog = $state(false);
	let showSelfDevDialog = $state(false);
	let showDevDeleteConfirm = $state(false);
	let devDeleteType: 'lab' | 'self' = $state('lab');

	// Lab dev form
	let devLabId = $state('');
	let devDateDroppedOff = $state('');
	let devDateReceived = $state('');
	let devCost = $state('');
	let devLabNotes = $state('');
	let devLabError = $state('');

	// Self dev form
	let devDateProcessed = $state('');
	let devDeveloper = $state('');
	let devDeveloperDilution = $state('');
	let devFixer = $state('');
	let devFixerDilution = $state('');
	let devStopBath = $state('');
	let devWettingAgent = $state('');
	let devClearingAgent = $state('');
	let devTemperature = $state('');
	let devAgitationNotes = $state('');
	let devSelfNotes = $state('');
	let devFormStages: { stage_name: string; duration: string; notes: string }[] = $state([]);
	let devSelfError = $state('');

	const labOptions = $derived([
		{ value: '', label: 'No lab selected' },
		...labs.map((l) => ({ value: String(l.id), label: l.name }))
	]);

	// Auto-prompt: parent sets autoPrompt to trigger opening a dialog
	$effect(() => {
		if (autoPrompt === 'lab') {
			openLabDevDialog();
			autoPrompt = null;
		} else if (autoPrompt === 'self') {
			openSelfDevDialog();
			autoPrompt = null;
		}
	});

	// --- Form helpers ---

	function resetLabDevForm() {
		devLabId = '';
		devDateDroppedOff = '';
		devDateReceived = '';
		devCost = '';
		devLabNotes = '';
		devLabError = '';
	}

	function resetSelfDevForm() {
		devDateProcessed = '';
		devDeveloper = '';
		devDeveloperDilution = '';
		devFixer = '';
		devFixerDilution = '';
		devStopBath = '';
		devWettingAgent = '';
		devClearingAgent = '';
		devTemperature = '';
		devAgitationNotes = '';
		devSelfNotes = '';
		devFormStages = [];
		devSelfError = '';
	}

	function openLabDevDialog() {
		if (labDev) {
			devLabId = labDev.lab_id?.toString() ?? '';
			devDateDroppedOff = labDev.date_dropped_off ?? '';
			devDateReceived = labDev.date_received ?? '';
			devCost = labDev.cost?.toString() ?? '';
			devLabNotes = labDev.notes ?? '';
		} else {
			resetLabDevForm();
			// Smart date default: last shot's date > roll's date_loaded > empty
			if (defaultDate) devDateDroppedOff = defaultDate;
		}
		devLabError = '';
		showLabDevDialog = true;
	}

	function openSelfDevDialog() {
		if (selfDev) {
			devDateProcessed = selfDev.date_processed ?? '';
			devDeveloper = selfDev.developer ?? '';
			devDeveloperDilution = selfDev.developer_dilution ?? '';
			devFixer = selfDev.fixer ?? '';
			devFixerDilution = selfDev.fixer_dilution ?? '';
			devStopBath = selfDev.stop_bath ?? '';
			devWettingAgent = selfDev.wetting_agent ?? '';
			devClearingAgent = selfDev.clearing_agent ?? '';
			devTemperature = selfDev.temperature ?? '';
			devAgitationNotes = selfDev.agitation_notes ?? '';
			devSelfNotes = selfDev.notes ?? '';
			devFormStages = devStages.map((s) => ({
				stage_name: s.stage_name,
				duration: secondsToMmSs(s.duration_seconds),
				notes: s.notes ?? ''
			}));
		} else {
			resetSelfDevForm();
			// Smart date default: last shot's date > roll's date_loaded > empty
			if (defaultDate) devDateProcessed = defaultDate;
		}
		devSelfError = '';
		showSelfDevDialog = true;
	}

	async function handleSaveLabDev() {
		devLabError = '';
		try {
			const payload = {
				roll_id: rollId,
				lab_id: devLabId ? Number(devLabId) : null,
				date_dropped_off: devDateDroppedOff || null,
				date_received: devDateReceived || null,
				cost: devCost ? parseFloat(devCost) : null,
				notes: devLabNotes || null
			};
			if (labDev) {
				const { roll_id, ...updatePayload } = payload;
				await updateLabDev(labDev.id, updatePayload);
			} else {
				await createLabDev(payload);
			}
			showLabDevDialog = false;
			resetLabDevForm();
			await onchange();
		} catch (err) {
			devLabError = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleSaveSelfDev() {
		devSelfError = '';
		try {
			const stages = devFormStages.map((s, i) => ({
				stage_name: s.stage_name,
				duration_seconds: mmSsToSeconds(s.duration),
				notes: s.notes || null,
				sort_order: i
			}));
			const payload = {
				roll_id: rollId,
				date_processed: devDateProcessed || null,
				developer: devDeveloper || null,
				developer_dilution: devDeveloperDilution || null,
				fixer: devFixer || null,
				fixer_dilution: devFixerDilution || null,
				stop_bath: devStopBath || null,
				wetting_agent: devWettingAgent || null,
				clearing_agent: devClearingAgent || null,
				temperature: devTemperature || null,
				agitation_notes: devAgitationNotes || null,
				notes: devSelfNotes || null,
				stages
			};
			if (selfDev) {
				const { roll_id, ...updatePayload } = payload;
				await updateSelfDev(selfDev.id, updatePayload);
			} else {
				await createSelfDev(payload);
			}
			showSelfDevDialog = false;
			resetSelfDevForm();
			await onchange();
		} catch (err) {
			devSelfError = err instanceof Error ? err.message : String(err);
		}
	}

	async function confirmDeleteDev() {
		try {
			if (devDeleteType === 'lab' && labDev) {
				await deleteLabDev(labDev.id);
			} else if (devDeleteType === 'self' && selfDev) {
				await deleteSelfDev(selfDev.id);
			}
			showDevDeleteConfirm = false;
			await onchange();
		} catch (err) {
			throw err;
		}
	}

	function addStage() {
		devFormStages = [...devFormStages, { stage_name: '', duration: '', notes: '' }];
	}

	function removeStage(index: number) {
		devFormStages = devFormStages.filter((_, i) => i !== index);
	}

	function moveStage(index: number, direction: -1 | 1) {
		const newIndex = index + direction;
		if (newIndex < 0 || newIndex >= devFormStages.length) return;
		const copy = [...devFormStages];
		[copy[index], copy[newIndex]] = [copy[newIndex], copy[index]];
		devFormStages = copy;
	}

	function getLabName(labId: number | null): string {
		if (!labId) return '';
		const lab = labs.find((l) => l.id === labId);
		return lab?.name ?? '';
	}
</script>

<!-- Development Display -->
<div class="mb-6">
	<div class="mb-3 flex items-center justify-between">
		<h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">Development</h2>
		{#if !labDev && !selfDev}
			<div class="flex gap-1">
				<Button size="sm" onclick={openLabDevDialog}>+ Lab</Button>
				<Button size="sm" onclick={openSelfDevDialog}>+ Self</Button>
			</div>
		{/if}
	</div>

	{#if labDev}
		<div class="group rounded-lg border border-border bg-surface-raised p-4">
			<div class="mb-2 flex items-center justify-between">
				<span class="text-xs font-semibold uppercase tracking-wider text-text-faint">Lab Development</span>
				<div class="flex gap-1 opacity-0 transition-opacity group-hover:opacity-100">
					<Button size="sm" variant="ghost" onclick={openLabDevDialog}>Edit</Button>
					<Button size="sm" variant="ghost" onclick={() => { devDeleteType = 'lab'; showDevDeleteConfirm = true; }}>&times;</Button>
				</div>
			</div>
			<div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
				{#if labDev.lab_id}
					<span class="text-text-muted">{getLabName(labDev.lab_id)}</span>
				{/if}
				{#if labDev.date_dropped_off}
					<span class="text-text-faint">Submitted: {labDev.date_dropped_off}</span>
				{/if}
				{#if labDev.date_received}
					<span class="text-text-faint">Received: {labDev.date_received}</span>
				{/if}
				{#if labDev.cost != null}
					<span class="text-text-faint">${labDev.cost.toFixed(2)}</span>
				{/if}
			</div>
			{#if labDev.notes}
				<p class="mt-1 text-xs text-text-faint">{labDev.notes}</p>
			{/if}
		</div>
	{:else if selfDev}
		<div class="group rounded-lg border border-border bg-surface-raised p-4">
			<div class="mb-2 flex items-center justify-between">
				<span class="text-xs font-semibold uppercase tracking-wider text-text-faint">Self Developed</span>
				<div class="flex gap-1 opacity-0 transition-opacity group-hover:opacity-100">
					<Button size="sm" variant="ghost" onclick={openSelfDevDialog}>Edit</Button>
					<Button size="sm" variant="ghost" onclick={() => { devDeleteType = 'self'; showDevDeleteConfirm = true; }}>&times;</Button>
				</div>
			</div>
			<div class="flex flex-wrap gap-x-4 gap-y-1 text-sm">
				{#if selfDev.date_processed}
					<span class="text-text-muted">{selfDev.date_processed}</span>
				{/if}
				{#if selfDev.temperature}
					<span class="text-text-faint">{selfDev.temperature}</span>
				{/if}
			</div>
			<div class="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-sm">
				{#if selfDev.developer}
					<span class="text-text-muted">
						{selfDev.developer}{selfDev.developer_dilution ? ` (${selfDev.developer_dilution})` : ''}
					</span>
				{/if}
				{#if selfDev.fixer}
					<span class="text-text-muted">
						Fixer: {selfDev.fixer}{selfDev.fixer_dilution ? ` (${selfDev.fixer_dilution})` : ''}
					</span>
				{/if}
				{#if selfDev.stop_bath}
					<span class="text-text-faint">Stop: {selfDev.stop_bath}</span>
				{/if}
			</div>
			{#if devStages.length > 0}
				<div class="mt-2 space-y-0.5">
					{#each devStages as stage, i}
						<div class="flex items-center gap-2 text-xs text-text-faint">
							<span class="font-mono text-text-muted">{i + 1}.</span>
							<span>{stage.stage_name}</span>
							{#if stage.duration_seconds}
								<span class="font-mono">{secondsToMmSs(stage.duration_seconds)}</span>
							{/if}
							{#if stage.notes}
								<span class="italic">{stage.notes}</span>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
			{#if selfDev.notes}
				<p class="mt-1 text-xs text-text-faint">{selfDev.notes}</p>
			{/if}
		</div>
	{:else}
		<p class="text-sm text-text-faint">No development info yet.</p>
	{/if}
</div>

<!-- Lab Development Dialog -->
{#if showLabDevDialog}
	<Dialog open={true} title={labDev ? 'Edit Lab Development' : 'Lab Development'} onclose={() => { showLabDevDialog = false; resetLabDevForm(); }}>
		<div class="space-y-4">
			<Select label="Lab" bind:value={devLabId} options={labOptions} />
			<div class="grid grid-cols-2 gap-3">
				<DateInput label="Date Submitted" bind:value={devDateDroppedOff} />
				<DateInput label="Date Received" bind:value={devDateReceived} />
			</div>
			<Input label="Cost" bind:value={devCost} placeholder="15.00" />
			<Textarea label="Notes" bind:value={devLabNotes} placeholder="Processing notes..." />

			{#if devLabError}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{devLabError}</div>
			{/if}
			<div class="flex justify-end gap-2 pt-2">
				<Button variant="ghost" onclick={() => { showLabDevDialog = false; resetLabDevForm(); }}>Cancel</Button>
				<Button variant="primary" onclick={handleSaveLabDev}>Save</Button>
			</div>
		</div>
	</Dialog>
{/if}

<!-- Self Development Dialog -->
{#if showSelfDevDialog}
	<Dialog open={true} title={selfDev ? 'Edit Self Development' : 'Self Development'} onclose={() => { showSelfDevDialog = false; resetSelfDevForm(); }}>
		<div class="space-y-4">
			<div class="grid grid-cols-2 gap-3">
				<DateInput label="Date Processed" bind:value={devDateProcessed} />
				<Input label="Temperature" bind:value={devTemperature} placeholder="20°C" />
			</div>

			<div>
				<span class="mb-2 block text-xs font-semibold uppercase tracking-wider text-text-faint">Chemistry</span>
				<div class="grid grid-cols-2 gap-3">
					<Input label="Developer" bind:value={devDeveloper} placeholder="Rodinal" />
					<Input label="Dilution" bind:value={devDeveloperDilution} placeholder="1+25" />
					<Input label="Fixer" bind:value={devFixer} placeholder="Ilford Rapid Fix" />
					<Input label="Dilution" bind:value={devFixerDilution} placeholder="1+4" />
					<Input label="Stop Bath" bind:value={devStopBath} placeholder="Optional" />
					<Input label="Clearing Agent" bind:value={devClearingAgent} placeholder="Optional" />
				</div>
				<div class="mt-3">
					<Input label="Wetting Agent" bind:value={devWettingAgent} placeholder="Optional" />
				</div>
			</div>

			<Textarea label="Agitation" bind:value={devAgitationNotes} placeholder="e.g., Continuous first 30s, then 3 inversions every 30s" />

			<div>
				<div class="mb-2 flex items-center justify-between">
					<span class="text-xs font-semibold uppercase tracking-wider text-text-faint">Stages</span>
					<Button size="sm" variant="ghost" onclick={addStage}>+ Add</Button>
				</div>
				{#if devFormStages.length > 0}
					<div class="space-y-2 rounded-lg border border-border bg-surface p-3">
						{#each devFormStages as stage, i}
							<div class="flex items-center gap-2">
								<span class="w-5 text-center font-mono text-xs text-text-faint">{i + 1}</span>
								<input
									bind:value={stage.stage_name}
									placeholder="Stage name"
									class="flex-1 rounded border border-border bg-surface px-2 py-1 text-sm text-text focus:border-accent focus:ring-1 focus:ring-accent/50 focus:outline-none"
								/>
								<input
									bind:value={stage.duration}
									placeholder="m:ss"
									class="w-16 rounded border border-border bg-surface px-2 py-1 text-center font-mono text-sm text-text focus:border-accent focus:ring-1 focus:ring-accent/50 focus:outline-none"
								/>
								<input
									bind:value={stage.notes}
									placeholder="Notes"
									class="w-24 rounded border border-border bg-surface px-2 py-1 text-sm text-text-muted focus:border-accent focus:ring-1 focus:ring-accent/50 focus:outline-none"
								/>
								<div class="flex gap-0.5">
									<button onclick={() => moveStage(i, -1)} disabled={i === 0} class="rounded p-0.5 text-text-faint transition-colors hover:bg-surface-overlay hover:text-text disabled:opacity-30"><ChevronUp size={12} /></button>
									<button onclick={() => moveStage(i, 1)} disabled={i === devFormStages.length - 1} class="rounded p-0.5 text-text-faint transition-colors hover:bg-surface-overlay hover:text-text disabled:opacity-30"><ChevronDown size={12} /></button>
									<button onclick={() => removeStage(i)} class="rounded p-0.5 text-text-faint transition-colors hover:bg-red-500/15 hover:text-red-400"><X size={12} /></button>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<p class="text-xs text-text-faint">No stages added. Click "+ Add" to add development steps.</p>
				{/if}
			</div>

			<Textarea label="Notes" bind:value={devSelfNotes} placeholder="General development notes..." />

			{#if devSelfError}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{devSelfError}</div>
			{/if}
			<div class="flex justify-end gap-2 pt-2">
				<Button variant="ghost" onclick={() => { showSelfDevDialog = false; resetSelfDevForm(); }}>Cancel</Button>
				<Button variant="primary" onclick={handleSaveSelfDev}>Save</Button>
			</div>
		</div>
	</Dialog>
{/if}

<!-- Delete Development Confirmation -->
{#if showDevDeleteConfirm}
	<ConfirmDialog
		open={true}
		title="Delete Development Info"
		message="Permanently delete this development record? This cannot be undone."
		confirmLabel="Delete"
		onconfirm={confirmDeleteDev}
		oncancel={() => { showDevDeleteConfirm = false; }}
	/>
{/if}
