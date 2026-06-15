import { describe, expect, it } from 'vitest';
import { mmSsToSeconds, secondsToMmSs } from './duration';

describe('secondsToMmSs', () => {
	it('returns an empty string for null', () => {
		expect(secondsToMmSs(null)).toBe('');
	});

	it('formats with zero-padded seconds', () => {
		expect(secondsToMmSs(0)).toBe('0:00');
		expect(secondsToMmSs(5)).toBe('0:05');
		expect(secondsToMmSs(65)).toBe('1:05');
		expect(secondsToMmSs(600)).toBe('10:00');
	});
});

describe('mmSsToSeconds', () => {
	it('returns null for empty/whitespace input', () => {
		expect(mmSsToSeconds('')).toBeNull();
		expect(mmSsToSeconds('   ')).toBeNull();
	});

	it('parses "m:ss" form', () => {
		expect(mmSsToSeconds('1:05')).toBe(65);
		expect(mmSsToSeconds('10:00')).toBe(600);
	});

	it('parses a bare minutes form as whole minutes', () => {
		expect(mmSsToSeconds('90')).toBe(5400);
	});

	it('returns null for non-numeric or malformed input', () => {
		expect(mmSsToSeconds('a:b')).toBeNull();
		expect(mmSsToSeconds('1:0:0')).toBeNull();
	});

	it('round-trips with secondsToMmSs', () => {
		expect(mmSsToSeconds(secondsToMmSs(125))).toBe(125);
	});
});
