import { describe, expect, it } from 'vitest';
import {
	devKindForStatus,
	getDevPath,
	getFlowForPath,
	getPathLabel,
	getStatusColor,
	getStatusColorSafe,
	getStatusLabel,
	isRollStatus,
	labFlow,
	selfFlow,
	undecidedFlow
} from './status';

describe('getDevPath', () => {
	it('prefers an existing dev record over the status', () => {
		expect(getDevPath('shot', true, false)).toBe('lab');
		expect(getDevPath('shot', false, true)).toBe('self');
		// A record wins even if the status implies the other path (data takes priority).
		expect(getDevPath('developing', true, false)).toBe('lab');
	});

	it('falls back to the status intrinsic path when no record exists (orphan)', () => {
		expect(getDevPath('at-lab', false, false)).toBe('lab');
		expect(getDevPath('developed', false, false)).toBe('self');
	});

	it('returns undecided for a roll-owned status with no record', () => {
		expect(getDevPath('loaded', false, false)).toBe('undecided');
		expect(getDevPath('scanned', false, false)).toBe('undecided');
	});
});

describe('devKindForStatus', () => {
	it('maps lab/self statuses and returns null for roll-owned ones', () => {
		expect(devKindForStatus('at-lab')).toBe('lab');
		expect(devKindForStatus('lab-done')).toBe('lab');
		expect(devKindForStatus('developing')).toBe('self');
		expect(devKindForStatus('developed')).toBe('self');
		expect(devKindForStatus('shot')).toBeNull();
		expect(devKindForStatus('archived')).toBeNull();
	});
});

describe('getFlowForPath / getPathLabel', () => {
	it('returns the fixture-derived flow array for each path', () => {
		expect(getFlowForPath('lab')).toBe(labFlow);
		expect(getFlowForPath('self')).toBe(selfFlow);
		expect(getFlowForPath('undecided')).toBe(undecidedFlow);
	});

	it('exposes the canonical flow contents', () => {
		expect(labFlow).toContain('at-lab');
		expect(labFlow).not.toContain('developing');
		expect(selfFlow).toContain('developing');
		expect(selfFlow).not.toContain('at-lab');
		// undecided skips both dev legs.
		expect(undecidedFlow).toEqual(['loaded', 'shooting', 'shot', 'scanned', 'post-processed', 'archived']);
	});

	it('labels lab/self paths and returns null for undecided', () => {
		expect(getPathLabel('lab')).toBe('Lab Development');
		expect(getPathLabel('self')).toBe('Self Development');
		expect(getPathLabel('undecided')).toBeNull();
	});
});

describe('status lookups', () => {
	it('is a type guard for known statuses', () => {
		expect(isRollStatus('shot')).toBe(true);
		expect(isRollStatus('not-a-status')).toBe(false);
	});

	it('resolves color + label for a known status', () => {
		expect(getStatusColor('loaded')).toBe('var(--color-status-loaded)');
		expect(getStatusLabel('post-processed')).toBe('Post-processed');
	});

	it('falls back to the accent color for an unknown status string', () => {
		expect(getStatusColorSafe('archived')).toBe('var(--color-status-archived)');
		expect(getStatusColorSafe('mystery')).toBe('var(--color-accent)');
	});
});
