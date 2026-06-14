import type { RollEvent } from '$lib/types';

/** A rendered journal row: either a single event, or a per-day rollup of shot events. */
export type ActivityRow =
	| { kind: 'event'; event: RollEvent }
	| { kind: 'shots'; day: string; count: number; latest: RollEvent };

export interface ActivityDay {
	day: string; // YYYY-MM-DD (from occurred_at)
	rows: ActivityRow[];
}

const SHOT_TYPES = new Set(['shot_logged', 'shot_edited', 'shot_deleted']);

/** The calendar day of an event (occurred_at is "YYYY-MM-DD HH:MM:SS"). */
function dayOf(e: RollEvent): string {
	return e.occurred_at.slice(0, 10);
}

/**
 * Group newest-first events into day buckets. Within a day, consecutive shot
 * events collapse into one `shots` rollup row ("N frame changes"); status/dev/
 * roll events stay as individual rows. Order within a day preserves the
 * incoming newest-first order.
 */
export function groupActivity(events: RollEvent[]): ActivityDay[] {
	const days: ActivityDay[] = [];
	let currentDay: ActivityDay | null = null;

	for (const e of events) {
		const day = dayOf(e);
		if (!currentDay || currentDay.day !== day) {
			currentDay = { day, rows: [] };
			days.push(currentDay);
		}
		if (SHOT_TYPES.has(e.event_type)) {
			const last = currentDay.rows[currentDay.rows.length - 1];
			if (last && last.kind === 'shots') {
				last.count += 1;
			} else {
				currentDay.rows.push({ kind: 'shots', day, count: 1, latest: e });
			}
		} else {
			currentDay.rows.push({ kind: 'event', event: e });
		}
	}
	return days;
}
