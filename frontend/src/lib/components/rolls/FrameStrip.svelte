<script lang="ts">
	import type { Shot } from '$lib/types';
	import { formatShotRow } from '$lib/utils/shot-table';

	interface FrameCell {
		frameNumber: string;
		shot: Shot | null;
		isNext: boolean;
	}

	interface Props {
		frames: FrameCell[];
		onselect: (frameNumber: string, shot: Shot | null) => void;
		onaddextra: () => void;
		/** Whether clicking a FILLED frame does something on this page. Pass false
		 * where it's a no-op (quick-entry), so the label never advertises a click
		 * action the page doesn't have. */
		filledClickable?: boolean;
	}

	let { frames, onselect, onaddextra, filledClickable = true }: Props = $props();

	// ONE filled-frame hint builder for both the tooltip (title) and the
	// accessible name — two hand-maintained templates would let the sighted hint
	// and the screen-reader one drift. Exposure values come from the shared
	// formatShotRow convention (f/ prefix, s suffix), same as the table/view.
	function filledHint(cell: FrameCell, sep: string, action = ''): string {
		const row = formatShotRow(cell.shot!);
		const parts = [row.date, row.aperture, row.shutter, row.time].filter(Boolean).join(sep);
		return `Frame ${cell.frameNumber}${parts ? sep + parts : ''}${action}`;
	}
</script>

<!-- Outer wrapper: the "film strip" surface with sprocket rails top & bottom -->
<div class="relative overflow-hidden rounded-lg border border-border bg-surface-raised">
	<!-- Top sprocket rail -->
	<div class="film-perfs-x h-3.5 w-full" aria-hidden="true"></div>

	<!-- Frame grid — wraps to multiple rows, no horizontal scroll -->
	<div class="px-2 py-1" role="group" aria-label="Film frames">
		<ul class="flex flex-wrap gap-1">
			{#each frames as cell}
				<li>
					<button
						onclick={() => onselect(cell.frameNumber, cell.shot)}
						title={cell.shot
							? filledHint(cell, ' · ')
							: cell.isNext
								? `Frame ${cell.frameNumber} — next open frame`
								: `Frame ${cell.frameNumber} — open`}
						aria-label={cell.shot
							? filledHint(cell, ', ', filledClickable ? ' — click to view' : '')
							: cell.isNext
								? `Frame ${cell.frameNumber}, next open frame — click to add`
								: `Frame ${cell.frameNumber}, open — click to add`}
						class="flex min-w-[3.5rem] flex-col items-center gap-0.5 rounded border px-1.5 py-2 text-center transition-all duration-150
					{cell.isNext
							? 'border-accent bg-accent/10 ring-1 ring-accent/50 hover:bg-accent/15'
							: cell.shot
								? 'border-border bg-surface hover:border-accent/40 hover:-translate-y-px'
								: 'border-border-subtle bg-surface hover:border-border hover:bg-surface-raised'}"
					>
						<!-- Frame number (mono, like a camera frame counter) -->
						<span
							class="font-mono text-[10px] font-medium leading-none
						{cell.shot || cell.isNext ? 'text-accent' : 'text-text-faint'}"
						>
							{cell.frameNumber}
						</span>
						<!-- Exposure hint when shot is logged -->
						{#if cell.shot}
							<!-- Date line (shown when date is present) -->
							{#if cell.shot.date}
								<span class="font-mono text-[9px] leading-tight text-text-faint">
									{cell.shot.date.slice(2)}
								</span>
							{/if}
							<span class="max-w-[3rem] truncate font-mono text-[9px] leading-tight text-text-muted">
								{#if cell.shot.aperture && cell.shot.shutter_speed}
									f/{cell.shot.aperture}/{cell.shot.shutter_speed}
								{:else if cell.shot.aperture}
									f/{cell.shot.aperture}
								{:else if cell.shot.shutter_speed}
									{cell.shot.shutter_speed}s
								{:else}
									<span class="text-text-faint">—</span>
								{/if}
							</span>
						{:else}
							<!-- Open slot: subtle dot indicator -->
							<span class="mt-0.5 h-1 w-1 rounded-full {cell.isNext ? 'bg-accent/60' : 'bg-border'}" aria-hidden="true"
							></span>
						{/if}
					</button>
				</li>
			{/each}

			<!-- Trailing ＋ button for over-roll / extra frames -->
			<li>
				<button
					onclick={onaddextra}
					title="Add an extra frame (over-roll or out-of-sequence)"
					aria-label="Add an extra frame"
					class="flex min-w-[3.5rem] flex-col items-center justify-center gap-0.5 rounded border border-dashed border-border-subtle px-1.5 py-2 text-center text-text-faint transition-colors duration-150 hover:border-border hover:text-text-muted"
				>
					<span class="font-mono text-base leading-none">＋</span>
				</button>
			</li>
		</ul>
	</div>

	<!-- Bottom sprocket rail -->
	<div class="film-perfs-x h-3.5 w-full" aria-hidden="true"></div>
</div>
