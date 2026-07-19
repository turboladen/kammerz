<script lang="ts">
	// Editor for the archiving activity's compound fields (ADR-0013): archiving is a
	// moment that is either Done (date + optional location), N/A (optional reason —
	// the no-negatives case), or Not yet. Emits a single normalized payload; the page
	// decides whether clearing a previously-set archive date needs a backward-move
	// confirmation. Wires onclose→reset per the form-dialog convention.
	import { untrack } from 'svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import { dateFieldError, todayLocal } from '$lib/utils/date';
	import type { ArchivePayload } from '$lib/utils/activity-board';

	interface Props {
		open: boolean;
		dateArchived: string | null;
		location: string | null;
		na: boolean;
		reason: string | null;
		onsave: (payload: ArchivePayload) => void;
		onclose: () => void;
	}

	let { open, dateArchived, location, na, reason, onsave, onclose }: Props = $props();

	type Choice = 'not_yet' | 'archived' | 'na';
	let choice = $state<Choice>('not_yet');
	let draftDate = $state('');
	let draftLocation = $state('');
	let draftReason = $state('');

	// Seed the form from the roll's current archive state each time the dialog opens.
	// The seed reads are untracked so this effect depends ONLY on `open` — otherwise
	// a prop update while the dialog is showing would re-seed and clobber the draft.
	$effect(() => {
		if (open) {
			untrack(() => {
				choice = dateArchived ? 'archived' : na ? 'na' : 'not_yet';
				draftDate = dateArchived ?? todayLocal();
				draftLocation = location ?? '';
				draftReason = reason ?? '';
			});
		}
	});

	const dateError = $derived(choice === 'archived' ? dateFieldError(draftDate) : '');
	const canSave = $derived(choice !== 'archived' || (!!draftDate.trim() && !dateError));

	function reset() {
		choice = 'not_yet';
		draftDate = '';
		draftLocation = '';
		draftReason = '';
	}

	function close() {
		reset();
		onclose();
	}

	function save() {
		if (choice === 'archived') {
			onsave({
				date_archived: draftDate.trim(),
				archive_location: draftLocation.trim() || null,
				archive_na: false,
				archive_na_reason: null
			});
		} else if (choice === 'na') {
			onsave({
				date_archived: null,
				archive_location: null,
				archive_na: true,
				archive_na_reason: draftReason.trim() || null
			});
		} else {
			onsave({ date_archived: null, archive_location: null, archive_na: false, archive_na_reason: null });
		}
		reset();
	}
</script>

{#if open}
	<Dialog open={true} title="Archiving" onclose={close}>
		<div class="space-y-4">
			<div class="flex flex-col gap-2">
				<!-- Shared `name` gives the group native radiogroup semantics (single tab
				     stop, arrow-key roving, "1 of 3" announcements) — bind:group alone
				     only enforces exclusivity in JS. -->
				<label class="flex items-center gap-2 text-sm text-text">
					<input type="radio" name="archive-choice" value="archived" bind:group={choice} class="accent-accent" /> Archived
				</label>
				<label class="flex items-center gap-2 text-sm text-text">
					<input type="radio" name="archive-choice" value="na" bind:group={choice} class="accent-accent" /> Not applicable
				</label>
				<label class="flex items-center gap-2 text-sm text-text">
					<input type="radio" name="archive-choice" value="not_yet" bind:group={choice} class="accent-accent" /> Not yet
				</label>
			</div>

			{#if choice === 'archived'}
				<Input type="date" label="Date archived" class="h-[38px]" bind:value={draftDate} />
				<Input label="Location" bind:value={draftLocation} placeholder="Binder 3, sleeve 12" />
			{:else if choice === 'na'}
				<Textarea label="Reason" bind:value={draftReason} placeholder="Lab discarded the negatives" />
			{/if}

			<div class="flex justify-end gap-2 pt-2">
				<Button variant="ghost" onclick={close}>Cancel</Button>
				<Button variant="primary" disabled={!canSave} onclick={save}>Save</Button>
			</div>
		</div>
	</Dialog>
{/if}
