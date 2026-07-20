<script lang="ts">
	import { groupActivity } from '$lib/utils/activity';
	import { formatLocalDayLabel, formatLocalTime } from '$lib/utils/datetime';
	import type { RollEvent } from '$lib/types';

	interface Props {
		events: RollEvent[];
		onopendev: (refKind: 'lab_dev' | 'self_dev') => void;
	}

	let { events, onopendev }: Props = $props();

	const days = $derived(groupActivity(events));

	// Day grouping and time rendering go through $lib/utils/datetime so the
	// naive-UTC occurred_at strings are shown in the browser's local timezone
	// (kammerz-j7s). `day.day` is already a LOCAL calendar day (from groupActivity).
	const formatDay = formatLocalDayLabel;
	const formatTime = formatLocalTime;

	/** Whether this event type represents a dev record that still exists (clickable). */
	function isDevClickable(eventType: RollEvent['event_type']): boolean {
		return (
			eventType === 'lab_dev_added' ||
			eventType === 'lab_dev_edited' ||
			eventType === 'self_dev_added' ||
			eventType === 'self_dev_edited'
		);
	}

	/** Which dev kind this event relates to, for the click handler. */
	function devRefKind(eventType: RollEvent['event_type']): 'lab_dev' | 'self_dev' | null {
		if (eventType === 'lab_dev_added' || eventType === 'lab_dev_edited') return 'lab_dev';
		if (eventType === 'self_dev_added' || eventType === 'self_dev_edited') return 'self_dev';
		return null;
	}
</script>

{#if days.length === 0}
	<div class="py-6 text-center text-sm text-text-faint">No activity yet.</div>
{:else}
	<div class="space-y-5">
		{#each days as day}
			<!-- Day header — ledger-line style -->
			<div>
				<h3 class="mb-2 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
					{formatDay(day.day)}
					<div class="flex-1 border-b border-border-subtle"></div>
				</h3>

				<ul class="space-y-1.5">
					{#each day.rows as row}
						<li>
							{#if row.kind === 'shots'}
								<!-- Shot rollup — quiet, faint -->
								<div class="flex items-center gap-2 px-1 py-0.5">
									<span class="h-1.5 w-1.5 flex-shrink-0 rounded-full bg-border" aria-hidden="true"></span>
									<span class="text-xs text-text-faint">
										{row.count} frame change{row.count > 1 ? 's' : ''}
									</span>
									<span class="ml-auto font-mono text-[10px] text-text-faint/60">
										{formatTime(row.latest.occurred_at)}
									</span>
								</div>
							{:else if row.event.event_type === 'roll_loaded'}
								<!-- Roll loaded — neutral dot -->
								<div class="flex items-center gap-2 px-1 py-0.5">
									<span class="h-2 w-2 flex-shrink-0 rounded-full bg-text-faint/40" aria-hidden="true"></span>
									<span class="text-xs text-text-muted">Roll loaded</span>
									<span class="ml-auto font-mono text-[10px] text-text-faint/60">
										{formatTime(row.event.occurred_at)}
									</span>
								</div>
							{:else if isDevClickable(row.event.event_type)}
								<!-- Dev add/edit — clickable button opens the dev editor -->
								{@const kind = devRefKind(row.event.event_type)}
								<button
									onclick={() => kind && onopendev(kind)}
									class="flex w-full items-center gap-2 rounded px-1 py-0.5 text-left transition-colors hover:bg-surface-overlay"
									title="Open {kind === 'lab_dev' ? 'lab' : 'self'} development details"
									aria-label="Open {kind === 'lab_dev' ? 'lab' : 'self'} development details"
								>
									<span class="h-2 w-2 flex-shrink-0 rounded-full bg-accent/60" aria-hidden="true"></span>
									<span class="text-xs text-text-muted hover:text-text">
										{row.event.summary}
									</span>
									<span class="ml-auto font-mono text-[10px] text-text-faint/60">
										{formatTime(row.event.occurred_at)}
									</span>
								</button>
							{:else}
								<!-- All other events (dev removed, shot events that aren't grouped, etc.) — non-clickable -->
								<div class="flex items-center gap-2 px-1 py-0.5">
									<span class="h-1.5 w-1.5 flex-shrink-0 rounded-full bg-border" aria-hidden="true"></span>
									<span class="text-xs text-text-faint">{row.event.summary}</span>
									<span class="ml-auto font-mono text-[10px] text-text-faint/60">
										{formatTime(row.event.occurred_at)}
									</span>
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		{/each}
	</div>
{/if}
