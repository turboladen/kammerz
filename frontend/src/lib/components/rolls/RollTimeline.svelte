<script lang="ts">
	// Inline-editable lifecycle timeline. Renders milestones (dot + label + dashed
	// rule + date) and lets the user set/change/clear each editable date via a
	// DateConfirm dialog. Emits onedit(milestone, date|null); the parent routes the
	// write to the roll/lab/self record. Pure UI — no data fetching here.
	import { Pencil } from 'lucide-svelte';
	import DateConfirm from '$lib/components/ui/DateConfirm.svelte';
	import { todayLocal } from '$lib/utils/date';
	import type { TimelineMilestone } from '$lib/utils/timeline';

	interface Props {
		milestones: TimelineMilestone[];
		onedit: (milestone: TimelineMilestone, date: string | null) => void;
	}

	let { milestones, onedit }: Props = $props();

	let editOpen = $state(false);
	let editing: TimelineMilestone | null = $state(null);

	function startEdit(m: TimelineMilestone) {
		// Order matters: set `editing` first so the {#if editing} block mounts
		// DateConfirm, then flip `editOpen` true so its $effect seeds the date on
		// mount. Reversing these would mount the dialog a frame before the seed.
		editing = m;
		editOpen = true;
	}
	function confirmEdit(date: string | null) {
		if (editing) onedit(editing, date);
		editOpen = false;
		editing = null;
	}
	function cancelEdit() {
		editOpen = false;
		editing = null;
	}
</script>

<ol class="space-y-1.5">
	{#each milestones as milestone (milestone.key)}
		<li class="flex items-center gap-3 text-sm">
			<span class="h-1.5 w-1.5 shrink-0 rounded-full {milestone.date ? 'bg-accent' : 'bg-surface-overlay'}"></span>
			<span class={milestone.date ? 'text-text-muted' : 'text-text-faint'}>{milestone.label}</span>
			<div class="flex-1 border-b border-dashed border-border-subtle/60"></div>
			{#if milestone.editable}
				<button
					type="button"
					onclick={() => startEdit(milestone)}
					class="group flex items-center gap-1.5 rounded px-1 py-0.5 transition-colors hover:bg-surface-overlay"
					aria-label={milestone.date ? `Edit ${milestone.label} date` : `Set ${milestone.label} date`}
				>
					<span class="font-mono text-xs {milestone.date ? 'text-text' : 'text-text-faint'}">
						{milestone.date ?? 'Set date'}
					</span>
					<Pencil size={12} strokeWidth={1.75} class="text-text-faint opacity-0 transition-opacity group-hover:opacity-100" />
				</button>
			{:else}
				<span class="font-mono text-xs text-text-faint">{milestone.date ?? '—'}</span>
			{/if}
		</li>
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
