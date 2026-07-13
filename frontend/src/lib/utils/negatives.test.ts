import { describe, expect, it } from 'vitest';
import { isNegativesPending, negativesState, type NegativesInput } from './negatives';

const base: NegativesInput = {
	negatives_date_received: '2026-07-01',
	negatives_deadline: '2026-07-31',
	date_negatives_picked_up: null,
	negatives_not_collecting: null
};
const on = (d: string) => new Date(`${d}T12:00:00`);

describe('negativesState', () => {
	it('is na when the order is not complete (no received date)', () => {
		const v = negativesState({ ...base, negatives_date_received: null, negatives_deadline: null }, on('2026-07-10'));
		expect(v.status).toBe('na');
		expect(v.tier).toBe('none');
		expect(v.label).toBe('');
	});

	it('is awaiting/far more than 7 days out', () => {
		const v = negativesState(base, on('2026-07-10')); // 21 days left
		expect(v.status).toBe('awaiting');
		expect(v.daysLeft).toBe(21);
		expect(v.tier).toBe('far');
		expect(v.label).toBe('21d left');
	});

	it('is near within 7 days', () => {
		const v = negativesState(base, on('2026-07-25')); // 6 days
		expect(v.tier).toBe('near');
	});

	it('is soon within 3 days', () => {
		const v = negativesState(base, on('2026-07-29')); // 2 days
		expect(v.tier).toBe('soon');
	});

	it('labels the deadline day itself as due today (still awaiting)', () => {
		const v = negativesState(base, on('2026-07-31'));
		expect(v.status).toBe('awaiting');
		expect(v.daysLeft).toBe(0);
		expect(v.label).toBe('Due today');
	});

	it('is overdue past the deadline', () => {
		const v = negativesState(base, on('2026-08-05')); // -5
		expect(v.status).toBe('overdue');
		expect(v.daysLeft).toBe(-5);
		expect(v.tier).toBe('overdue');
		expect(v.label).toBe('OVERDUE');
	});

	it('is picked-up when a pickup date is set', () => {
		const v = negativesState({ ...base, date_negatives_picked_up: '2026-07-05' }, on('2026-08-05'));
		expect(v.status).toBe('picked-up');
		expect(v.label).toBe('Collected');
		expect(v.tier).toBe('none');
	});

	it('is waived (takes priority over a pickup date)', () => {
		const v = negativesState(
			{ ...base, negatives_not_collecting: true, date_negatives_picked_up: '2026-07-05' },
			on('2026-08-05')
		);
		expect(v.status).toBe('waived');
		expect(v.label).toBe('Not collecting');
	});

	it('defaults null not_collecting to false', () => {
		const v = negativesState({ ...base, negatives_not_collecting: null }, on('2026-07-10'));
		expect(v.status).toBe('awaiting');
	});
});

describe('isNegativesPending', () => {
	it('is true only for awaiting and overdue', () => {
		expect(isNegativesPending(negativesState(base, on('2026-07-10')))).toBe(true); // awaiting
		expect(isNegativesPending(negativesState(base, on('2026-08-05')))).toBe(true); // overdue
		expect(
			isNegativesPending(negativesState({ ...base, date_negatives_picked_up: '2026-07-05' }, on('2026-08-05')))
		).toBe(false);
		expect(
			isNegativesPending(
				negativesState({ ...base, negatives_date_received: null, negatives_deadline: null }, on('2026-07-10'))
			)
		).toBe(false);
	});
});
