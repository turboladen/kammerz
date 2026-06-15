import { describe, expect, it } from 'vitest';
import { dateFieldError, todayLocal } from './date';

describe('dateFieldError', () => {
	it('accepts empty/nullish values (dates are optional)', () => {
		expect(dateFieldError('')).toBe('');
		expect(dateFieldError('   ')).toBe('');
		expect(dateFieldError(null)).toBe('');
		expect(dateFieldError(undefined)).toBe('');
	});

	it('accepts complete YYYY, YYYY-MM, and YYYY-MM-DD values', () => {
		expect(dateFieldError('2026')).toBe('');
		expect(dateFieldError('2026-06')).toBe('');
		expect(dateFieldError('2026-06-15')).toBe('');
		expect(dateFieldError('2024-02-29')).toBe(''); // real leap day
	});

	it('rejects years outside 1800–2100', () => {
		expect(dateFieldError('1799')).toBe('Year out of range');
		expect(dateFieldError('2101-01')).toBe('Year out of range');
	});

	it('rejects an out-of-range month', () => {
		expect(dateFieldError('2026-13')).toBe('Month must be 01–12');
		expect(dateFieldError('2026-00-01')).toBe('Month must be 01–12');
	});

	it('rejects a day that does not exist in the month', () => {
		expect(dateFieldError('2025-02-29')).toBe('Invalid day for this month'); // 2025 not a leap year
		expect(dateFieldError('2026-04-31')).toBe('Invalid day for this month');
	});

	it('rejects malformed / partial input', () => {
		expect(dateFieldError('2026-')).toBe('Use YYYY, YYYY-MM, or YYYY-MM-DD');
		expect(dateFieldError('06/15/2026')).toBe('Use YYYY, YYYY-MM, or YYYY-MM-DD');
		expect(dateFieldError('not-a-date')).toBe('Use YYYY, YYYY-MM, or YYYY-MM-DD');
	});
});

describe('todayLocal', () => {
	it('returns a zero-padded local YYYY-MM-DD that matches the local Date parts', () => {
		const s = todayLocal();
		expect(s).toMatch(/^\d{4}-\d{2}-\d{2}$/);
		const now = new Date();
		const expected = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(
			now.getDate()
		).padStart(2, '0')}`;
		expect(s).toBe(expected);
	});
});
