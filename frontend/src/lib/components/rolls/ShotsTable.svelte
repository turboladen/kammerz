<script lang="ts">
	import type { Shot } from '$lib/types';
	import { formatShotRow } from '$lib/utils/shot-table';

	interface Props {
		/** Shots in FrameStrip reading order (roll-detail's `orderedShots`). */
		shots: Shot[];
		/** Precomputed per-shot lens display, keyed by shot id. */
		lensNames: Record<number, string>;
		onselect: (shot: Shot) => void;
		onaddextra: () => void;
	}

	let { shots, lensNames, onselect, onaddextra }: Props = $props();
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
					<th class="py-2 pr-3 font-medium">f/</th>
					<th class="py-2 pr-3 font-medium">Shutter</th>
					<th class="py-2 pr-3 font-medium">Date</th>
					<th class="py-2 pr-3 font-medium">Time</th>
					<th class="py-2 pr-3 font-medium">Location</th>
					<th class="py-2 pr-3 font-medium">Lens</th>
					<th class="py-2 pr-3 font-medium">Notes</th>
				</tr>
			</thead>
			<tbody>
				{#each shots as shot (shot.id)}
					{@const row = formatShotRow(shot)}
					<!-- Row click is a pointer-only convenience; the Frame-cell button is the
					     sole keyboard/SR control. Both call the same idempotent onselect. -->
					<tr
						class="cursor-pointer border-b border-border-subtle align-top transition-colors last:border-b-0 hover:bg-surface-overlay"
						onclick={() => onselect(shot)}
					>
						<td class="py-2 pr-3 pl-3">
							<button
								class="rounded font-medium text-accent transition-colors hover:text-accent-hover focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/50"
								aria-label="View frame {row.frame}"
								onclick={(e) => {
									e.stopPropagation();
									onselect(shot);
								}}
							>
								{row.frame}
							</button>
						</td>
						<td class="py-2 pr-3 whitespace-nowrap text-text-muted">{row.aperture}</td>
						<td class="py-2 pr-3 whitespace-nowrap text-text-muted">{row.shutter}</td>
						<td class="py-2 pr-3 whitespace-nowrap text-text-muted">{row.date}</td>
						<td class="py-2 pr-3 whitespace-nowrap text-text-muted">{row.time}</td>
						<td class="py-2 pr-3 text-text-muted">{row.location}</td>
						<td class="py-2 pr-3 text-text-muted">{lensNames[shot.id] ?? ''}</td>
						<td class="py-2 pr-3 whitespace-pre-wrap break-words text-text-muted">{row.notes}</td>
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
