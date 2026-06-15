import { describe, expect, it } from 'vitest';
import type { Camera } from '$lib/types';
import { buildCameraLabels, buildDisambiguatedLabels } from './disambiguate';

interface Item {
	id: number;
	serial_number?: string | null;
	created_at: string;
}

const label = (i: Item & { name: string }) => i.name;

describe('buildDisambiguatedLabels', () => {
	it('leaves single-instance labels unchanged', () => {
		const items = [{ id: 1, created_at: '2026-01-01', name: 'Nikon F3' }];
		expect(buildDisambiguatedLabels(items, label).get(1)).toBe('Nikon F3');
	});

	it('appends "(S/N x)" when duplicates have serial numbers', () => {
		const items = [
			{ id: 1, created_at: '2026-01-01', serial_number: 'AAA', name: 'Nikon F3' },
			{ id: 2, created_at: '2026-01-02', serial_number: 'BBB', name: 'Nikon F3' }
		];
		const m = buildDisambiguatedLabels(items, label);
		expect(m.get(1)).toBe('Nikon F3 (S/N AAA)');
		expect(m.get(2)).toBe('Nikon F3 (S/N BBB)');
	});

	it('numbers serial-less duplicates as "(Copy N)" by created_at order', () => {
		// Deliberately out of chronological order to prove the sort drives numbering.
		const items = [
			{ id: 1, created_at: '2026-03-01', name: 'Leica M6' },
			{ id: 2, created_at: '2026-01-01', name: 'Leica M6' }
		];
		const m = buildDisambiguatedLabels(items, label);
		expect(m.get(2)).toBe('Leica M6 (Copy 1)'); // earlier created_at
		expect(m.get(1)).toBe('Leica M6 (Copy 2)');
	});

	it('mixes S/N and Copy numbering within one group (Copy counter ignores serialled items)', () => {
		const items = [
			{ id: 1, created_at: '2026-01-01', serial_number: 'SN1', name: 'RB67' },
			{ id: 2, created_at: '2026-01-02', name: 'RB67' },
			{ id: 3, created_at: '2026-01-03', name: 'RB67' }
		];
		const m = buildDisambiguatedLabels(items, label);
		expect(m.get(1)).toBe('RB67 (S/N SN1)');
		expect(m.get(2)).toBe('RB67 (Copy 1)');
		expect(m.get(3)).toBe('RB67 (Copy 2)');
	});
});

describe('buildCameraLabels', () => {
	const camera = (o: Partial<Camera>): Camera =>
		({ id: 1, brand: 'Nikon', model: 'F3', serial_number: null, created_at: '2026-01-01', ...o }) as Camera;

	it('labels cameras by "brand model" and disambiguates duplicates', () => {
		const m = buildCameraLabels([
			camera({ id: 1, brand: 'Nikon', model: 'F3' }),
			camera({ id: 2, brand: 'Nikon', model: 'F3', serial_number: '1234' }),
			camera({ id: 3, brand: 'Leica', model: 'M6' })
		]);
		expect(m.get(2)).toBe('Nikon F3 (S/N 1234)');
		expect(m.get(3)).toBe('Leica M6'); // unique → unchanged
	});
});
