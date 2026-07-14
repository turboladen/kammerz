import { describe, expect, it } from 'vitest';
import type { Chemical } from '$lib/types';
import { defaultDilutionFor, dilutionPrefill } from './chemistry';

const chem = (name: string, default_dilution: string | null): Chemical => ({
	id: 1,
	name,
	type: 'developer',
	default_dilution,
	created_at: '',
	updated_at: ''
});

describe('defaultDilutionFor', () => {
	const list = [chem('Kodak D-76', 'stock'), chem('Kodak XTOL', '1+1'), chem('FPP Monobath', null)];

	it('returns the default_dilution for a known chemical', () => {
		expect(defaultDilutionFor(list, 'Kodak XTOL')).toBe('1+1');
	});

	it('returns null for a chemical with no default', () => {
		expect(defaultDilutionFor(list, 'FPP Monobath')).toBeNull();
	});

	it('returns null for an unknown name', () => {
		expect(defaultDilutionFor(list, 'Nonexistent')).toBeNull();
	});
});

describe('dilutionPrefill', () => {
	it('fills an empty field from the default', () => {
		expect(dilutionPrefill('', '1+25')).toBe('1+25');
	});

	it('treats a whitespace-only current value as empty', () => {
		expect(dilutionPrefill('   ', 'stock')).toBe('stock');
	});

	it('never overwrites a non-empty current value', () => {
		expect(dilutionPrefill('1+50', '1+25')).toBeNull();
	});

	it('is a no-op when the chemical has no default', () => {
		expect(dilutionPrefill('', null)).toBeNull();
		expect(dilutionPrefill('', undefined)).toBeNull();
		expect(dilutionPrefill('', '  ')).toBeNull();
	});
});
