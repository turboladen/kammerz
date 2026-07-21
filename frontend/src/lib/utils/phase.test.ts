import { describe, it, expect } from 'vitest';
import { PHASE_META, phaseTheme, phaseLabel } from './phase';

// The canonical phase labels. PHASE_META is their ONLY home — the backend
// speaks group_key integers on every surface (stats buckets included), so this
// pin guards against accidental frontend renames/reorders, not a backend sync
// (no backend label list exists, and none should be added).
const EXPECTED_LABELS = ['Shooting', 'Development', 'Scanning', 'Post-processing', 'Archiving', 'Done'];

describe('PHASE_META', () => {
	it('covers group_key 0..=5 in order', () => {
		expect(PHASE_META).toHaveLength(6);
		PHASE_META.forEach((p, i) => {
			expect(p.groupKey).toBe(i);
		});
	});

	it('matches the canonical phase labels', () => {
		expect(PHASE_META.map((p) => p.label)).toEqual(EXPECTED_LABELS);
	});

	it('gives every phase a colorVar, pill classes, and a dot class', () => {
		for (const p of PHASE_META) {
			expect(p.colorVar).toMatch(/^var\(--color-status-[a-z-]+\)$/);
			expect(p.pillClasses).toContain('text-status-');
			expect(p.dotClass).toMatch(/^bg-status-/);
		}
	});
});

describe('phaseTheme', () => {
	it('returns the entry for an in-range group_key', () => {
		expect(phaseTheme(1).label).toBe('Development');
	});

	it('clamps an out-of-range group_key to Done', () => {
		expect(phaseTheme(99).label).toBe('Done');
		expect(phaseTheme(-1).label).toBe('Done');
	});
});

describe('phaseLabel', () => {
	it('maps group_key to its phase name', () => {
		expect(phaseLabel(0)).toBe('Shooting');
		expect(phaseLabel(5)).toBe('Done');
	});
});
