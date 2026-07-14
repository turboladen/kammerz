import { describe, expect, it } from 'vitest';
import { coerceApproxDate, dateFieldError, todayLocal } from './date';

describe('dateFieldError', () => {
	it('accepts empty/nullish values (dates are optional)', () => {
		expect(dateFieldError('')).toBe('');
		expect(dateFieldError('   ')).toBe('');
		expect(dateFieldError(null)).toBe('');
		expect(dateFieldError(undefined)).toBe('');
	});

	it('accepts a complete YYYY-MM-DD value', () => {
		expect(dateFieldError('2026-06-15')).toBe('');
		expect(dateFieldError('2024-02-29')).toBe(''); // real leap day
	});

	it('rejects partial dates (YYYY / YYYY-MM) — full dates only', () => {
		expect(dateFieldError('2026')).toBe('Use YYYY-MM-DD');
		expect(dateFieldError('2026-06')).toBe('Use YYYY-MM-DD');
	});

	it('rejects years outside 1800–2100', () => {
		expect(dateFieldError('1799-01-01')).toBe('Year out of range');
		expect(dateFieldError('2101-01-01')).toBe('Year out of range');
	});

	it('rejects an out-of-range month', () => {
		expect(dateFieldError('2026-13-01')).toBe('Month must be 01–12');
		expect(dateFieldError('2026-00-01')).toBe('Month must be 01–12');
	});

	it('rejects a day that does not exist in the month', () => {
		expect(dateFieldError('2025-02-29')).toBe('Invalid day for this month'); // 2025 not a leap year
		expect(dateFieldError('2026-04-31')).toBe('Invalid day for this month');
	});

	it('rejects malformed input', () => {
		expect(dateFieldError('2026-')).toBe('Use YYYY-MM-DD');
		expect(dateFieldError('06/15/2026')).toBe('Use YYYY-MM-DD');
		expect(dateFieldError('not-a-date')).toBe('Use YYYY-MM-DD');
	});
});

describe('coerceApproxDate', () => {
	it('returns empty with no note for empty/nullish input', () => {
		expect(coerceApproxDate('')).toEqual({ date: '', note: null });
		expect(coerceApproxDate('   ')).toEqual({ date: '', note: null });
		expect(coerceApproxDate(null)).toEqual({ date: '', note: null });
		expect(coerceApproxDate(undefined)).toEqual({ date: '', note: null });
	});

	it('passes a valid full date through unchanged with no note', () => {
		expect(coerceApproxDate('2026-06-15')).toEqual({ date: '2026-06-15', note: null });
		expect(coerceApproxDate('2024-02-29')).toEqual({ date: '2024-02-29', note: null }); // leap day
		expect(coerceApproxDate('  1998-03-04  ')).toEqual({ date: '1998-03-04', note: null }); // trimmed
	});

	it('coerces a year-only value to Jan 1 + an approx note', () => {
		expect(coerceApproxDate('1998')).toEqual({ date: '1998-01-01', note: 'approx date: 1998' });
	});

	it('coerces a year-month value to the 1st + an approx note', () => {
		expect(coerceApproxDate('1998-05')).toEqual({ date: '1998-05-01', note: 'approx date: 1998-05' });
	});

	it('passes an unparseable value through unchanged so dateFieldError can flag it', () => {
		expect(coerceApproxDate('5/16/21')).toEqual({ date: '5/16/21', note: null });
		expect(coerceApproxDate('not-a-date')).toEqual({ date: 'not-a-date', note: null });
		// A value that isn't best-guessable must stay visible as an error, not vanish.
		expect(dateFieldError('5/16/21')).not.toBe('');
	});

	it('passes an out-of-range or invalid partial through unchanged rather than emitting a bad date', () => {
		// Year out of the 1800–2100 window: the Jan-1 candidate fails dateFieldError.
		expect(coerceApproxDate('1700')).toEqual({ date: '1700', note: null });
		expect(dateFieldError('1700')).not.toBe('');
		// Impossible month in a year-month partial.
		expect(coerceApproxDate('1998-13')).toEqual({ date: '1998-13', note: null });
		expect(dateFieldError('1998-13')).not.toBe('');
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
