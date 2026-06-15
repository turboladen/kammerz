import { describe, expect, it } from 'vitest';
import type { DevelopmentLab, DevelopmentSelf, RollWithDetails } from '$lib/types';
import { readDateTarget, STATUS_DATE_TARGET } from './timeline';

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
