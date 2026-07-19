<script lang="ts">
	// The roll's lifecycle as five activity rows (ADR-0013), replacing the old
	// chevron status control + Lifecycle-dates section. Purely presentational: it
	// renders the server-derived activity states and emits edit intents. The page
	// owns every dialog (DateConfirm / ConfirmDialog / ArchiveDialog) and the dev
	// dialog bridge, so this component renders no Dialog of its own — safe to sit
	// inside FadeIn.
	import { Pencil, X, ChevronDown, ChevronUp } from 'lucide-svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import {
		activityLabel,
		isDatedKind,
		SLOT_CAPTIONS,
		slotDateLabel,
		stateLabel,
		type DatedActivityKind,
		type DateSlot
	} from '$lib/utils/activity-board';
	import type { ActivityKind, ActivityState, RollActivityItem } from '$lib/types';

	interface Props {
		activities: RollActivityItem[];
		/** Whether a lab/self dev record exists — drives the Development row's
		 * Start-vs-Edit affordance independently of the derived state. */
		hasDevRecord: boolean;
		/** Compound badge string for the collapsed summary. */
		badge: string;
		expanded: boolean;
		archiveLocation: string | null;
		archiveNaReason: string | null;
		onToggleExpanded: () => void;
		/** Edit (set/change) a roll-owned date; page opens DateConfirm. */
		oneditdate: (kind: ActivityKind, slot: DateSlot) => void;
		/** Clear a set roll-owned date; page confirms (backward move). */
		oncleardate: (kind: ActivityKind, slot: DateSlot) => void;
		/** Start development on a chosen path; page opens the dev dialog. */
		onchoosepath: (path: 'lab' | 'self') => void;
		/** Open the development dialog for an existing record. */
		onopendev: () => void;
		/** Open the archiving editor (date + location / N-A + reason). */
		oneditarchiving: () => void;
	}

	let {
		activities,
		hasDevRecord,
		badge,
		expanded,
		archiveLocation,
		archiveNaReason,
		onToggleExpanded,
		oneditdate,
		oncleardate,
		onchoosepath,
		onopendev,
		oneditarchiving
	}: Props = $props();

	function dotClass(state: ActivityState): string {
		switch (state) {
			case 'done':
				return 'bg-accent';
			case 'in_progress':
				return 'bg-accent/50';
			case 'na':
				return 'bg-text-faint/40';
			case 'not_started':
				return 'bg-surface-overlay';
		}
	}
</script>

{#snippet dateControl(kind: DatedActivityKind, slot: DateSlot, date: string | null)}
	<!-- Accessible names use slotDateLabel ("Set Scanning started date") — the same
	     phrase source as the page's DateConfirm titles and clear-confirm labels:
	     scanning and post-processing share the 'Started' caption, so caption-only
	     names would collide for screen readers and role-based test locators. -->
	<div class="flex items-center gap-1">
		<button
			type="button"
			onclick={() => oneditdate(kind, slot)}
			class="group flex items-center gap-1.5 rounded px-1 py-0.5 transition-colors hover:bg-surface-overlay"
			aria-label={`${date ? 'Edit' : 'Set'} ${slotDateLabel(kind, slot)}`}
		>
			<span class="font-mono text-xs {date ? 'text-text' : 'text-text-faint'}">{date ?? 'Set date'}</span>
			<Pencil
				size={12}
				strokeWidth={1.75}
				class="text-text-faint opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 pointer-coarse:opacity-100"
			/>
		</button>
		{#if date}
			<button
				type="button"
				onclick={() => oncleardate(kind, slot)}
				class="rounded p-0.5 text-text-faint transition-colors hover:bg-red-500/15 hover:text-red-400"
				aria-label={`Clear ${slotDateLabel(kind, slot)}`}
			>
				<X size={12} strokeWidth={2} aria-hidden="true" />
			</button>
		{/if}
	</div>
{/snippet}

{#snippet datedSlot(kind: DatedActivityKind, slot: DateSlot, date: string | null)}
	<div class="flex items-center gap-1.5">
		<span class="text-[10px] font-medium uppercase tracking-wider text-text-faint">{SLOT_CAPTIONS[kind][slot]}</span>
		{@render dateControl(kind, slot, date)}
	</div>
{/snippet}

<div>
	<div class="mb-3 flex items-center gap-3">
		<h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">Activity</h2>
		<div class="flex-1 border-b border-border-subtle"></div>
		<button
			type="button"
			onclick={onToggleExpanded}
			aria-expanded={expanded}
			aria-label={expanded ? 'Collapse activity board' : 'Expand activity board'}
			class="inline-flex items-center text-text-faint transition-colors hover:text-text"
		>
			{#if expanded}
				<ChevronUp size={16} strokeWidth={2} aria-hidden="true" />
			{:else}
				<ChevronDown size={16} strokeWidth={2} aria-hidden="true" />
			{/if}
		</button>
	</div>

	{#if !expanded}
		<button
			type="button"
			onclick={onToggleExpanded}
			aria-expanded={expanded}
			class="flex w-full items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-2.5 text-left transition-colors hover:border-accent/40"
		>
			<span class="text-sm text-text-muted">{badge}</span>
			<span class="text-xs text-text-faint">Show details</span>
		</button>
	{:else}
		<div class="divide-y divide-border-subtle rounded-lg border border-border bg-surface-raised px-4">
			{#each activities as a (a.kind)}
				<div class="flex flex-wrap items-center gap-x-4 gap-y-1.5 py-2.5">
					<div class="flex w-36 items-center gap-2">
						<span class="h-2 w-2 shrink-0 rounded-full {dotClass(a.state)}"></span>
						<span class="text-sm text-text">{activityLabel(a.kind)}</span>
					</div>
					<span class="w-24 text-xs {a.state === 'not_started' ? 'text-text-faint' : 'text-text-muted'}">
						{stateLabel(a.kind, a.state)}
					</span>
					<div class="flex flex-1 flex-wrap items-center justify-end gap-x-4 gap-y-1">
						{#if isDatedKind(a.kind)}
							{@render datedSlot(a.kind, 'start', a.start)}
							{@render datedSlot(a.kind, 'completion', a.completion)}
						{:else if a.kind === 'development'}
							<!-- Branch on RECORD presence alone: implicit completion (any tail
							     date) marks development done with NO record, and hiding the
							     Lab/Self choice there forecloses recording a lab dev without
							     destroying the tail date (kammerz-73to). not_started implies
							     no record (both mirror the backend's has_dev), so no state
							     check is needed — and record-presence reacts instantly to the
							     dev dialog's bindable writes, where derived state lags a
							     reload. The caption says "Record" when the dev already
							     happened (state is past not_started) so the row never reads
							     "Done" next to a "Start" affordance. -->
							{#if !hasDevRecord}
								<span class="text-[10px] font-medium uppercase tracking-wider text-text-faint">
									{a.state === 'not_started' ? 'Start' : 'Record'}
								</span>
								<Button size="sm" variant="secondary" onclick={() => onchoosepath('lab')}>Lab</Button>
								<Button size="sm" variant="secondary" onclick={() => onchoosepath('self')}>Self</Button>
							{:else}
								<!-- Only a done dev has a completion date; an in-progress row must not
								     caption its empty slot "Completed" while stating "In progress". -->
								{#if a.completion}
									<div class="flex items-center gap-1.5">
										<span class="text-[10px] font-medium uppercase tracking-wider text-text-faint">Completed</span>
										<span class="font-mono text-xs text-text">{a.completion}</span>
									</div>
								{/if}
								<Button size="sm" variant="ghost" onclick={onopendev}>Edit</Button>
							{/if}
						{:else if a.kind === 'archiving'}
							{#if a.state === 'done'}
								<div class="flex items-center gap-1.5">
									<span class="text-[10px] font-medium uppercase tracking-wider text-text-faint">Archived</span>
									<span class="font-mono text-xs text-text">{a.completion}</span>
									{#if archiveLocation}
										<span class="text-xs text-text-faint">· {archiveLocation}</span>
									{/if}
								</div>
								<Button size="sm" variant="ghost" onclick={oneditarchiving}>Edit</Button>
							{:else if a.state === 'na'}
								<div class="flex items-center gap-1.5">
									<span class="text-xs text-text-muted">Not applicable</span>
									{#if archiveNaReason}
										<span class="text-xs text-text-faint">· {archiveNaReason}</span>
									{/if}
								</div>
								<Button size="sm" variant="ghost" onclick={oneditarchiving}>Edit</Button>
							{:else}
								<Button size="sm" variant="secondary" onclick={oneditarchiving}>Edit archiving</Button>
							{/if}
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
