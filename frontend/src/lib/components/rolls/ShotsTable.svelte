<script lang="ts">
	import type { Shot } from '$lib/types';
	import { formatShotRow, type ShotRowDisplay } from '$lib/utils/shot-table';

	interface Props {
		/** Shots in FrameStrip reading order (roll-detail's `orderedShots`). */
		shots: Shot[];
		/** Precomputed per-shot lens display, keyed by shot id. */
		lensNames: Record<number, string>;
		onselect: (shot: Shot) => void;
		onaddextra: () => void;
	}

	let { shots, lensNames, onselect, onaddextra }: Props = $props();

	// One definition per column drives BOTH the header row and the body cells, so
	// they can never fall out of lockstep (a hand-maintained pair silently shifts
	// every value under the wrong header when one side is edited).
	const COLUMNS: { label: string; cls: string; value: (row: ShotRowDisplay, shot: Shot) => string }[] = [
		{ label: 'Aperture', cls: 'whitespace-nowrap', value: (r) => r.aperture },
		{ label: 'Shutter', cls: 'whitespace-nowrap', value: (r) => r.shutter },
		{ label: 'Date', cls: 'whitespace-nowrap', value: (r) => r.date },
		{ label: 'Time', cls: 'whitespace-nowrap', value: (r) => r.time },
		{ label: 'Location', cls: '', value: (r) => r.location },
		{ label: 'Lens', cls: '', value: (_r, shot) => lensNames[shot.id] ?? '' },
		{ label: 'Notes', cls: 'whitespace-pre-wrap break-words', value: (r) => r.notes }
	];
</script>

{#if shots.length === 0}
	<p class="rounded-lg border border-border bg-surface-raised px-4 py-3 text-sm text-text-faint">
		No shots logged yet.
	</p>
{:else}
	<!-- Wide content (notes/location) scrolls in its own container — the page body
	     never scrolls horizontally (UI_DESIGN). -->
	<div class="overflow-x-auto rounded-lg border border-border bg-surface-raised">
		<table class="w-full min-w-[44rem] border-collapse font-mono text-xs">
			<thead>
				<tr class="border-b border-border text-left text-text-faint">
					<th class="py-2 pr-3 pl-3 font-medium">#</th>
					{#each COLUMNS as col (col.label)}
						<th class="py-2 pr-3 font-medium">{col.label}</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each shots as shot (shot.id)}
					{@const row = formatShotRow(shot)}
					<!-- Rows are deliberately NOT click targets: the table exists for
					     zero-click reading and text selection (transcribing metadata), and a
					     whole-row onclick would open the dialog on any select/copy attempt.
					     The Frame-cell button is the sole open affordance. -->
					<tr
						class="border-b border-border-subtle align-top transition-colors last:border-b-0 hover:bg-surface-overlay"
					>
						<td class="py-2 pr-3 pl-3">
							<button
								class="rounded font-medium text-accent transition-colors hover:text-accent-hover focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/50"
								aria-label="View frame {row.frame}"
								onclick={() => onselect(shot)}
							>
								{row.frame}
							</button>
						</td>
						{#each COLUMNS as col (col.label)}
							<td class="py-2 pr-3 text-text-muted {col.cls}">{col.value(row, shot)}</td>
						{/each}
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
{/if}

<div class="mt-2">
	<button
		onclick={onaddextra}
		class="inline-flex items-center gap-1.5 rounded-lg border border-dashed border-border-subtle px-3 py-1.5 text-xs text-text-faint transition-colors hover:border-border hover:text-text-muted"
		aria-label="Add a frame"
	>
		<span class="font-mono text-sm leading-none">＋</span>
		Add a frame
	</button>
</div>
