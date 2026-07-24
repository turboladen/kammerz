import { describe, it, expect } from 'vitest';
import { safeNext } from './redirect';

const ORIGIN = 'http://localhost';

describe('safeNext', () => {
	it('returns / for a null next', () => {
		expect(safeNext(null, ORIGIN)).toBe('/');
	});

	it('returns / for an empty next', () => {
		expect(safeNext('', ORIGIN)).toBe('/');
	});

	it('rejects a protocol-relative //host', () => {
		expect(safeNext('//evil.com', ORIGIN)).toBe('/');
	});

	it('rejects a backslash host (\\ resolves like /)', () => {
		expect(safeNext('/\\evil.com', ORIGIN)).toBe('/');
	});

	it('rejects a control-char host (tab stripped → //host)', () => {
		expect(safeNext('/\t/evil.com', ORIGIN)).toBe('/');
	});

	it('rejects a cross-origin absolute URL', () => {
		expect(safeNext('https://evil.com/steal', ORIGIN)).toBe('/');
	});

	it('rejects a login loop (/login)', () => {
		expect(safeNext('/login', ORIGIN)).toBe('/');
	});

	it('rejects a login loop even with a query (/login?x=1)', () => {
		expect(safeNext('/login?x=1', ORIGIN)).toBe('/');
	});

	it('accepts a same-origin path, preserving search and hash', () => {
		expect(safeNext('/rolls?x=1#h', ORIGIN)).toBe('/rolls?x=1#h');
	});

	it('accepts a bare same-origin path', () => {
		expect(safeNext('/rolls', ORIGIN)).toBe('/rolls');
	});
});
