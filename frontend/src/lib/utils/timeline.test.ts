import { describe, expect, it } from 'vitest';
import type { DevelopmentLab, DevelopmentSelf, RollWithDetails } from '$lib/types';
import { buildRollTimeline, dateTargetEditable, readDateTarget, STATUS_DATE_TARGET } from './timeline';
import type { MilestoneKey, TimelineMilestone } from './timeline';

const roll = (o: Partial<RollWithDetails>): RollWithDetails => ({ ...o }) as RollWithDetails;
const labDev = (o: Partial<DevelopmentLab>): DevelopmentLab => ({ ...o }) as DevelopmentLab;
const selfDev = (o: Partial<DevelopmentSelf>): DevelopmentSelf => ({ ...o }) as DevelopmentSelf;

describe('readDateTarget', () => {
	it('reads a roll-owned field from the roll', () => {
		const r = roll({ date_loaded: '2026-06-01' });
		expect(readDateTarget({ kind: 'roll', field: 'date_loaded' }, r, null, null)).toBe('2026-06-01');
	});

	it('returns null for a roll field that is unset', () => {
		expect(readDateTarget({ kind: 'roll', field: 'date_archived' }, roll({}), null, null)).toBeNull();
	});

	it('reads a lab field from the lab dev record', () => {
		const lab = labDev({ date_received: '2026-06-10' });
		expect(readDateTarget({ kind: 'lab', field: 'date_received' }, roll({}), lab, null)).toBe('2026-06-10');
	});

	it('returns null for a lab/self target when the dev record is absent', () => {
		expect(readDateTarget({ kind: 'lab', field: 'date_received' }, roll({}), null, null)).toBeNull();
		expect(readDateTarget({ kind: 'self', field: 'date_processed' }, roll({}), null, null)).toBeNull();
	});

	it('reads a self field from the self dev record', () => {
		const self = selfDev({ date_processed: '2026-06-12' });
		expect(readDateTarget({ kind: 'self', field: 'date_processed' }, roll({}), null, self)).toBe('2026-06-12');
	});
});

describe('STATUS_DATE_TARGET', () => {
	it('routes each milestone status to the record + field that owns its date', () => {
		expect(STATUS_DATE_TARGET['shot']).toEqual({ kind: 'roll', field: 'date_finished' });
		expect(STATUS_DATE_TARGET['lab-done']).toEqual({ kind: 'lab', field: 'date_received' });
		expect(STATUS_DATE_TARGET['developed']).toEqual({ kind: 'self', field: 'date_processed' });
		expect(STATUS_DATE_TARGET['scanned']).toEqual({ kind: 'roll', field: 'date_scanned' });
		expect(STATUS_DATE_TARGET['archived']).toEqual({ kind: 'roll', field: 'date_archived' });
	});

	it('has no target for intermediate, date-less statuses', () => {
		expect(STATUS_DATE_TARGET['loaded']).toBeUndefined();
		expect(STATUS_DATE_TARGET['shooting']).toBeUndefined();
		expect(STATUS_DATE_TARGET['at-lab']).toBeUndefined();
		expect(STATUS_DATE_TARGET['developing']).toBeUndefined();
	});
});

describe('dateTargetEditable', () => {
	it('always allows a roll-owned target, even with no dev records', () => {
		expect(dateTargetEditable({ kind: 'roll', field: 'date_loaded' }, null, null)).toBe(true);
	});

	it('allows a lab/self target only once its dev record exists', () => {
		expect(dateTargetEditable({ kind: 'lab', field: 'date_received' }, null, null)).toBe(false);
		expect(dateTargetEditable({ kind: 'lab', field: 'date_received' }, labDev({}), null)).toBe(true);
		expect(dateTargetEditable({ kind: 'self', field: 'date_processed' }, null, null)).toBe(false);
		expect(dateTargetEditable({ kind: 'self', field: 'date_processed' }, null, selfDev({}))).toBe(true);
	});
});

describe('buildRollTimeline', () => {
	const keys = (ms: TimelineMilestone[]) => ms.map((m) => m.key);
	const find = (ms: TimelineMilestone[], key: MilestoneKey) => ms.find((m) => m.key === key)!;

	it('emits the lab-path milestone set in order', () => {
		const ms = buildRollTimeline(roll({ status: 'lab-done' }), labDev({}), null, 'lab');
		expect(keys(ms)).toEqual([
			'loaded',
			'finished-shooting',
			'dropped-off',
			'received',
			'scanned',
			'post-processed',
			'archived'
		]);
	});

	it('emits the self-path milestone set in order', () => {
		const ms = buildRollTimeline(roll({ status: 'developed' }), null, selfDev({}), 'self');
		expect(keys(ms)).toEqual(['loaded', 'finished-shooting', 'developed', 'scanned', 'post-processed', 'archived']);
	});

	it('omits the dev middle on the undecided path', () => {
		const ms = buildRollTimeline(roll({ status: 'shooting' }), null, null, 'undecided');
		expect(keys(ms)).toEqual(['loaded', 'finished-shooting', 'scanned', 'post-processed', 'archived']);
	});

	it('reads each milestone date from the record that owns it', () => {
		const ms = buildRollTimeline(
			roll({ status: 'lab-done', date_loaded: '2026-06-01' }),
			labDev({ date_received: '2026-06-10' }),
			null,
			'lab'
		);
		expect(find(ms, 'loaded').date).toBe('2026-06-01');
		expect(find(ms, 'received').date).toBe('2026-06-10');
	});

	it('marks a reached, roll-owned milestone editable', () => {
		const ms = buildRollTimeline(roll({ status: 'shot' }), null, null, 'undecided');
		expect(find(ms, 'finished-shooting').editable).toBe(true);
	});

	it('keeps a reached milestone editable even after its date was cleared', () => {
		const ms = buildRollTimeline(roll({ status: 'shot', date_finished: null }), null, null, 'undecided');
		const m = find(ms, 'finished-shooting');
		expect(m.date).toBeNull();
		expect(m.editable).toBe(true);
	});

	it('leaves a not-yet-reached milestone non-editable even if its date somehow exists', () => {
		const ms = buildRollTimeline(roll({ status: 'loaded', date_scanned: '2026-07-01' }), null, null, 'undecided');
		const m = find(ms, 'scanned');
		expect(m.date).toBe('2026-07-01');
		expect(m.editable).toBe(false);
	});

	it('leaves a reached lab/self milestone non-editable until its dev record exists', () => {
		// Roll pushed onto the lab path (e.g. via import) but with no lab dev record yet:
		// 'received' is reached by status but has no backing record, so it stays read-only.
		const ms = buildRollTimeline(roll({ status: 'lab-done' }), null, null, 'lab');
		expect(find(ms, 'received').editable).toBe(false);
	});

	it('marks a reached lab milestone editable once the lab dev record exists', () => {
		const ms = buildRollTimeline(roll({ status: 'lab-done' }), labDev({}), null, 'lab');
		expect(find(ms, 'received').editable).toBe(true);
	});
});
