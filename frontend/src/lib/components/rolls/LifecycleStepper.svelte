<script lang="ts">
	// Unified lifecycle stepper — the single view of a roll's progression AND its
	// dated milestones (merges the former STATUS chevron bar + TIMELINE list,
	// kammerz-06i). Each rung is a status: clicking the row drives the transition
	// (parent's handleStatusClick — forward instant, backward confirm, lab/self
	// opens a dev dialog, date-bearing forward asks for a date), while the date on
	// a reached rung is click-to-edit via DateConfirm. Future / dateless rungs show
	// an em-dash with no editor (the date is recorded by advancing to that rung).
	// Pure UI apart from the self-contained inline date-edit dialog.
	import { Pencil } from 'lucide-svelte';
	import DateConfirm from '$lib/components/ui/DateConfirm.svelte';
	import { todayLocal } from '$lib/utils/date';
	import type { LifecycleRung, TimelineMilestone } from '$lib/utils/timeline';
	import type { DevPath } from '$lib/utils/status';
	import type { RollStatus } from '$lib/types';

	interface Props {
		rungs: LifecycleRung[];
		devPath: DevPath;
		/** Title/aria text for a rung's transition action — mirrors the click behavior. */
		hintFor: (status: RollStatus) => string;
		onmove: (status: RollStatus) => void;
		oneditdate: (milestone: TimelineMilestone, date: string | null) => void;
		onchoosepath: (path: 'lab' | 'self') => void;
	}

	let { rungs, devPath, hintFor, onmove, oneditdate, onchoosepath }: Props = $props();

	let editOpen = $state(false);
	let editing: TimelineMilestone | null = $state(null);
	let showDevMenu = $state(false);

	function startEdit(m: TimelineMilestone) {
		// Set `editing` first so DateConfirm mounts, then open it so its seed $effect runs.
		editing = m;
		editOpen = true;
	}
	function confirmEdit(date: string | null) {
		if (editing) oneditdate(editing, date);
		editOpen = false;
		editing = null;
	}
	function cancelEdit() {
		editOpen = false;
		editing = null;
	}

	function choose(path: 'lab' | 'self') {
		showDevMenu = false;
		onchoosepath(path);
	}

	// The development decision lives as a live next-step after "Shot" on the
	// undecided path — choosing a path opens the matching dev dialog and re-renders
	// the stepper onto that flow (lab or self), rather than dead placeholder rungs.
	const showChooserAfterShot = $derived(devPath === 'undecided');
</script>

<ol>
	{#each rungs as rung, i (rung.status)}
		{@const last = i === rungs.length - 1 && !(showChooserAfterShot && rung.status === 'shot')}
		<li class="flex gap-3">
			<!-- Rail: state dot + connector to the next rung -->
			<div class="flex flex-col items-center" aria-hidden="true">
				<span
					class="mt-1.5 h-2.5 w-2.5 shrink-0 rounded-full {rung.state === 'current'
						? 'bg-accent ring-2 ring-accent/30'
						: rung.state === 'past'
							? 'bg-accent'
							: 'border border-border bg-surface-raised'}"
				></span>
				{#if !last}
					<span
						class="w-px flex-1 {rung.state === 'past' || rung.state === 'current'
							? 'bg-accent/40'
							: 'bg-border-subtle'}"
					></span>
				{/if}
			</div>

			<!-- Body: transition row (label) + the rung's date -->
			<div class="flex flex-1 items-center justify-between gap-3 pb-4">
				<button
					type="button"
					onclick={() => onmove(rung.status)}
					title={hintFor(rung.status)}
					aria-label={hintFor(rung.status)}
					aria-current={rung.state === 'current' ? 'step' : undefined}
					class="-mx-1 rounded px-1 py-0.5 text-left text-sm transition-colors hover:bg-surface-overlay/60
					{rung.state === 'current' ? 'font-medium text-text' : rung.state === 'past' ? 'text-text-muted' : 'text-text-faint'}"
				>
					{rung.label}
				</button>

				{#if rung.milestone?.editable}
					<button
						type="button"
						onclick={() => startEdit(rung.milestone!)}
						class="group flex items-center gap-1.5 rounded px-1 py-0.5 transition-colors hover:bg-surface-overlay"
						aria-label={rung.milestone.date ? `Edit ${rung.milestone.label} date` : `Set ${rung.milestone.label} date`}
					>
						<span class="font-mono text-xs {rung.milestone.date ? 'text-text' : 'text-text-faint'}">
							{rung.milestone.date ?? 'Set date'}
						</span>
						<Pencil
							size={12}
							strokeWidth={1.75}
							class="pointer-coarse:opacity-100 text-text-faint opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100"
						/>
					</button>
				{:else}
					<span class="font-mono text-xs text-text-faint">{rung.milestone?.date ?? '—'}</span>
				{/if}
			</div>
		</li>

		{#if showChooserAfterShot && rung.status === 'shot'}
			<!-- Live development-path chooser (undecided path), sitting between Shot and Scanned. -->
			<li class="flex gap-3">
				<div class="flex flex-col items-center" aria-hidden="true">
					<span class="mt-1.5 h-2.5 w-2.5 shrink-0 rounded-full border border-dashed border-accent/60"></span>
					<span class="w-px flex-1 bg-border-subtle"></span>
				</div>
				<div class="flex flex-1 items-center pb-4">
					<div class="relative">
						<button
							type="button"
							onclick={() => (showDevMenu = !showDevMenu)}
							title="Choose a development path (lab or self) to start tracking development"
							aria-haspopup="menu"
							aria-expanded={showDevMenu}
							class="rounded px-2 py-1 text-xs font-medium transition-colors hover:bg-accent/25
							{showDevMenu ? 'bg-accent/25 text-accent' : 'bg-accent/15 text-accent'}"
						>
							Develop<span aria-hidden="true">&nbsp;⌄</span>
						</button>
						{#if showDevMenu}
							<!-- click-away catcher -->
							<button
								class="fixed inset-0 z-10 cursor-default"
								aria-label="Close development menu"
								onclick={() => (showDevMenu = false)}
							></button>
							<div
								role="menu"
								class="absolute left-0 top-full z-20 mt-1.5 overflow-hidden rounded-lg border border-border bg-surface-overlay shadow-lg"
							>
								<button
									role="menuitem"
									onclick={() => choose('lab')}
									class="block w-full whitespace-nowrap px-4 py-2 text-left text-xs font-medium text-text-muted transition-colors hover:bg-accent/15 hover:text-accent"
									>Lab</button
								>
								<button
									role="menuitem"
									onclick={() => choose('self')}
									class="block w-full whitespace-nowrap border-t border-border-subtle px-4 py-2 text-left text-xs font-medium text-text-muted transition-colors hover:bg-accent/15 hover:text-accent"
									>Self / Home</button
								>
							</div>
						{/if}
					</div>
				</div>
			</li>
		{/if}
	{/each}
</ol>

{#if editing}
	<DateConfirm
		bind:open={editOpen}
		title={editing.date ? `Edit "${editing.label}" date` : `Set "${editing.label}" date`}
		value={editing.date ?? todayLocal()}
		confirmLabel="Save"
		allowClear={!!editing.date}
		onconfirm={confirmEdit}
		oncancel={cancelEdit}
	/>
{/if}
