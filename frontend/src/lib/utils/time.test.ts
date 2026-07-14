import { describe, expect, it } from 'vitest';
import { parseTime, timeFieldError } from './time';

describe('parseTime', () => {
	it('returns "" for blank/whitespace', () => {
		expect(parseTime('')).toBe('');
		expect(parseTime('   ')).toBe('');
	});

	it('canonicalizes HH:MM and H:MM to zero-padded 24-hour', () => {
		expect(parseTime('14:30')).toBe('14:30');
		expect(parseTime('9:05')).toBe('09:05');
		expect(parseTime('00:00')).toBe('00:00');
		expect(parseTime('23:59')).toBe('23:59');
	});

	it('accepts colon-less 4-digit entry', () => {
		expect(parseTime('1430')).toBe('14:30');
		expect(parseTime('0705')).toBe('07:05');
	});

	it('rejects out-of-range hours/minutes', () => {
		expect(parseTime('24:00')).toBeNull();
		expect(parseTime('14:60')).toBeNull();
		expect(parseTime('99:99')).toBeNull();
	});

	it('rejects 12-hour / non-time junk', () => {
		expect(parseTime('3:00 PM')).toBeNull();
		expect(parseTime('abc')).toBeNull();
		expect(parseTime('14')).toBeNull(); // bare hour is not a complete time
		expect(parseTime('143')).toBeNull(); // ambiguous 3-digit
	});
});

describe('timeFieldError', () => {
	it('is empty for blank and for valid times', () => {
		expect(timeFieldError('')).toBe('');
		expect(timeFieldError('14:30')).toBe('');
		expect(timeFieldError('1430')).toBe('');
		expect(timeFieldError('9:05')).toBe('');
	});

	it('does not nag while the hour/colon is still being typed', () => {
		expect(timeFieldError('1')).toBe('');
		expect(timeFieldError('14')).toBe('');
		expect(timeFieldError('14:')).toBe('');
		expect(timeFieldError('14:3')).toBe('');
	});

	it('errors on a completed but invalid entry', () => {
		expect(timeFieldError('25:00')).toBe('Use 24-hour HH:MM');
		expect(timeFieldError('3:00 PM')).toBe('Use 24-hour HH:MM');
		expect(timeFieldError('14:99')).toBe('Use 24-hour HH:MM');
	});
});
