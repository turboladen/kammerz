import { describe, expect, it } from 'vitest';
import {
	APERTURE_SUGGESTIONS,
	SHUTTER_SUGGESTIONS,
	isRecognizedAperture,
	isRecognizedShutter,
	normalizeAperture,
	normalizeShutter
} from './exposure';

describe('normalizeAperture', () => {
	it('strips a leading f/ or f', () => {
		expect(normalizeAperture('f/5.6')).toBe('5.6');
		expect(normalizeAperture('F5.6')).toBe('5.6');
	});
	it('converts comma to dot and removes whitespace', () => {
		expect(normalizeAperture('5,6')).toBe('5.6');
		expect(normalizeAperture('  2.8 ')).toBe('2.8');
	});
	it('leaves a bare value unchanged and is idempotent', () => {
		expect(normalizeAperture('5.6')).toBe('5.6');
		expect(normalizeAperture(normalizeAperture('f/5.6'))).toBe('5.6');
	});
	it('returns empty for empty input', () => {
		expect(normalizeAperture('')).toBe('');
	});
});

describe('normalizeShutter', () => {
	it('strips a trailing s', () => {
		expect(normalizeShutter('1/250s')).toBe('1/250');
		expect(normalizeShutter('30 s')).toBe('30');
	});
	it('leaves fractions, seconds, and B untouched', () => {
		expect(normalizeShutter('1/250')).toBe('1/250');
		expect(normalizeShutter('4')).toBe('4');
		expect(normalizeShutter('B')).toBe('B');
	});
	it('returns empty for empty input', () => {
		expect(normalizeShutter('')).toBe('');
	});
});

describe('isRecognizedAperture', () => {
	it('recognizes full/half/third-stop values (normalizing first)', () => {
		expect(isRecognizedAperture('5.6')).toBe(true);
		expect(isRecognizedAperture('1.8')).toBe(true);
		expect(isRecognizedAperture('3.5')).toBe(true);
		expect(isRecognizedAperture('64')).toBe(true);
		expect(isRecognizedAperture('f/5.6')).toBe(true);
	});
	it('flags genuine typos as non-standard', () => {
		expect(isRecognizedAperture('56')).toBe(false);
		expect(isRecognizedAperture('8.5')).toBe(false);
	});
	it('is false for empty', () => {
		expect(isRecognizedAperture('')).toBe(false);
	});
});

describe('isRecognizedShutter', () => {
	it('recognizes standard and legacy speeds (normalizing first)', () => {
		expect(isRecognizedShutter('1/250')).toBe(true);
		expect(isRecognizedShutter('1/50')).toBe(true);
		expect(isRecognizedShutter('B')).toBe(true);
		expect(isRecognizedShutter('1/250s')).toBe(true);
	});
	it('flags genuine typos as non-standard', () => {
		expect(isRecognizedShutter('250')).toBe(false);
		expect(isRecognizedShutter('1/275')).toBe(false);
	});
	it('is false for empty', () => {
		expect(isRecognizedShutter('')).toBe(false);
	});
});

describe('suggestion lists are bare', () => {
	it('have no f/ prefix and no trailing s', () => {
		expect(APERTURE_SUGGESTIONS).toContain('5.6');
		expect(APERTURE_SUGGESTIONS.every((v) => !v.toLowerCase().startsWith('f'))).toBe(true);
		expect(SHUTTER_SUGGESTIONS).toContain('1/250');
		expect(SHUTTER_SUGGESTIONS.every((v) => !v.endsWith('s'))).toBe(true);
	});
});
