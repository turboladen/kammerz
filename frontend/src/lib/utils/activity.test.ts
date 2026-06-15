import { describe, expect, it } from 'vitest';
import type { RollEvent } from '$lib/types';
import { groupActivity } from './activity';

let seq = 0;
const ev = (event_type: string, occurred_at: string): RollEvent =>
	({ id: ++seq, event_type, occurred_at }) as RollEvent;

describe('groupActivity', () => {
	it('returns an empty array for no events', () => {
		expect(groupActivity([])).toEqual([]);
	});

	it('buckets events by calendar day (from occurred_at)', () => {
		const days = groupActivity([
			ev('status_changed', '2026-06-15 10:00:00'),
			ev('status_changed', '2026-06-14 09:00:00')
		]);
		expect(days.map((d) => d.day)).toEqual(['2026-06-15', '2026-06-14']);
	});

	it('collapses consecutive shot events within a day into one rollup row with a count', () => {
		const days = groupActivity([
			ev('shot_logged', '2026-06-15 12:00:00'),
			ev('shot_edited', '2026-06-15 11:00:00'),
			ev('shot_deleted', '2026-06-15 10:00:00')
		]);
		expect(days).toHaveLength(1);
		expect(days[0].rows).toHaveLength(1);
		const row = days[0].rows[0];
		expect(row.kind).toBe('shots');
		if (row.kind === 'shots') {
			expect(row.count).toBe(3);
			expect(row.latest.event_type).toBe('shot_logged'); // first (newest) seeds the rollup
		}
	});

	it('keeps non-shot events as individual rows and breaks the shot rollup', () => {
		const days = groupActivity([
			ev('shot_logged', '2026-06-15 12:00:00'),
			ev('status_changed', '2026-06-15 11:30:00'),
			ev('shot_logged', '2026-06-15 11:00:00')
		]);
		const kinds = days[0].rows.map((r) => r.kind);
		// shot → event → shot, so the two shot rows do NOT merge across the status event.
		expect(kinds).toEqual(['shots', 'event', 'shots']);
	});
});
