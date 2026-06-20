import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { localDay, formatLocalDayLabel, formatLocalTime, parseUtcNaive } from './datetime';

// These tests pin a fixed timezone so the local-conversion assertions are
// deterministic regardless of where CI runs. `process.env.TZ` is read by V8's
// Intl/Date when set before the Date is constructed.
const ORIGINAL_TZ = process.env.TZ;

describe('parseUtcNaive', () => {
	it('parses a naive "YYYY-MM-DD HH:MM:SS" string as UTC (not local)', () => {
		const d = parseUtcNaive('2026-06-19 22:17:30');
		// Asserted in UTC so it is timezone-independent: the string is UTC.
		expect(d.getUTCFullYear()).toBe(2026);
		expect(d.getUTCMonth()).toBe(5); // June (0-indexed)
		expect(d.getUTCDate()).toBe(19);
		expect(d.getUTCHours()).toBe(22);
		expect(d.getUTCMinutes()).toBe(17);
		expect(d.getUTCSeconds()).toBe(30);
	});

	it('also accepts an ISO "T"-separated string', () => {
		const d = parseUtcNaive('2026-06-19T22:17:30');
		expect(d.getUTCHours()).toBe(22);
		expect(d.getUTCMinutes()).toBe(17);
	});
});

describe('in America/Los_Angeles (PDT, UTC-7)', () => {
	beforeEach(() => {
		process.env.TZ = 'America/Los_Angeles';
	});
	afterEach(() => {
		process.env.TZ = ORIGINAL_TZ;
		vi.useRealTimers();
	});

	it('formats a UTC timestamp as the local wall-clock time', () => {
		// 2026-06-19 22:17:30 UTC == 2026-06-19 15:17 PDT
		expect(formatLocalTime('2026-06-19 22:17:30')).toBe('15:17');
	});

	it('rolls back to the previous local day when UTC has already crossed midnight', () => {
		// 2026-06-20 00:12:00 UTC == 2026-06-19 17:12 PDT — the repro from kammerz-j7s.
		expect(formatLocalTime('2026-06-20 00:12:00')).toBe('17:12');
		expect(localDay('2026-06-20 00:12:00')).toBe('2026-06-19');
	});

	it('localDay returns the local calendar day, not the UTC day', () => {
		// Same instant as above: UTC day is the 20th, local day is the 19th.
		expect(localDay('2026-06-20 06:00:00')).toBe('2026-06-19');
	});

	it('formats a readable local day label', () => {
		// 2026-06-20 00:12 UTC is Friday June 19 locally (PDT).
		expect(formatLocalDayLabel('2026-06-19')).toContain('June');
		expect(formatLocalDayLabel('2026-06-19')).toContain('19');
	});
});

describe('in UTC', () => {
	beforeEach(() => {
		process.env.TZ = 'UTC';
	});
	afterEach(() => {
		process.env.TZ = ORIGINAL_TZ;
	});

	it('is an identity transform when the local zone IS UTC', () => {
		expect(formatLocalTime('2026-06-19 22:17:30')).toBe('22:17');
		expect(localDay('2026-06-19 22:17:30')).toBe('2026-06-19');
	});
});

describe('formatLocalDayLabel', () => {
	it('renders a readable label for a valid local day', () => {
		const label = formatLocalDayLabel('2026-06-19');
		expect(label).toContain('June');
		expect(label).toContain('19');
	});

	it('falls back to the raw string for an unparseable day (not "Invalid Date")', () => {
		expect(formatLocalDayLabel('garbage')).toBe('garbage');
	});
});
