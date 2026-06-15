import { describe, expect, it } from 'vitest';
import type { Lens, LensMount } from '$lib/types';
import { buildLensOptions, buildMountOptions, isLargeFormatMount, lensDisplayName, NEW_MOUNT_OPTION } from './lens';

const lens = (o: Partial<Lens>): Lens =>
	({
		id: 1,
		brand: 'Nikon',
		model: null,
		focal_length: null,
		max_aperture: null,
		lens_mount_id: 1,
		date_sold: null,
		serial_number: null,
		created_at: '2026-01-01',
		...o
	}) as Lens;

const mount = (id: number, name: string): LensMount =>
	({ id, name, created_at: '2026-01-01', updated_at: '2026-01-01' }) as LensMount;

describe('lensDisplayName', () => {
	it('joins brand and model', () => {
		expect(lensDisplayName({ brand: 'Nikon', model: 'Nikkor 50mm' })).toBe('Nikon Nikkor 50mm');
	});

	it('does not double the brand when model already starts with it', () => {
		expect(lensDisplayName({ brand: 'Mamiya', model: 'Mamiya 90mm K/L' })).toBe('Mamiya 90mm K/L');
	});

	it('falls back to brand + focal length + aperture when model is empty', () => {
		expect(lensDisplayName({ brand: 'Nikon', model: '', focal_length: '50', max_aperture: '1.4' })).toBe(
			'Nikon 50mm f/1.4'
		);
	});
});

describe('isLargeFormatMount', () => {
	it('recognises the large-format mount families', () => {
		for (const name of ['Copal #0', 'Compur #1', 'Barrel Mount', 'Generic Large Format']) {
			expect(isLargeFormatMount(name)).toBe(true);
		}
	});

	it('rejects non-LF mounts', () => {
		expect(isLargeFormatMount('Nikon F')).toBe(false);
		expect(isLargeFormatMount('Fixed Lens')).toBe(false);
	});
});

describe('buildMountOptions', () => {
	const mounts = [mount(1, 'Nikon F'), mount(2, 'Hasselblad V'), mount(3, 'Copal #0'), mount(4, 'Fixed Lens')];

	it('groups by format family with disabled dividers and a trailing "+ New mount" sentinel', () => {
		const opts = buildMountOptions(mounts, 'Pick one');
		expect(opts[0]).toEqual({ value: '', label: 'Pick one' });

		const dividers = opts.filter((o) => o.disabled).map((o) => o.label);
		expect(dividers).toEqual(['── 35mm ──', '── Medium Format ──', '── Large Format ──', '── Other ──', '── New ──']);

		// The 35mm group lists Nikon F before the Medium Format group's Hasselblad.
		const labels = opts.map((o) => o.label);
		expect(labels.indexOf('Nikon F')).toBeLessThan(labels.indexOf('Hasselblad V'));

		const last = opts[opts.length - 1];
		expect(last).toEqual({ value: NEW_MOUNT_OPTION, label: '+ New mount…' });
	});

	it('omits a group divider when that group is empty', () => {
		const opts = buildMountOptions([mount(1, 'Nikon F')]);
		const dividers = opts.filter((o) => o.disabled).map((o) => o.label);
		expect(dividers).toEqual(['── 35mm ──', '── New ──']);
	});
});

describe('buildLensOptions', () => {
	it('excludes sold lenses', () => {
		const opts = buildLensOptions(
			[lens({ id: 1, model: 'Kept' }), lens({ id: 2, model: 'Sold', date_sold: '2026-01-01' })],
			null
		);
		expect(opts.map((o) => o.label)).not.toContain('Nikon Sold');
		expect(opts.some((o) => o.value === '2')).toBe(false);
	});

	it('lists compatible lenses first, then a divider, then the rest', () => {
		const lenses = [
			lens({ id: 1, model: 'Match A', lens_mount_id: 1 }),
			lens({ id: 2, model: 'Other', lens_mount_id: 99 }),
			lens({ id: 3, model: 'Match B', lens_mount_id: 1 })
		];
		const opts = buildLensOptions(lenses, { lens_mount_id: 1 });
		const labels = opts.map((o) => o.label);
		expect(labels.indexOf('Nikon Match A')).toBeLessThan(labels.indexOf('── Other lenses ──'));
		expect(labels.indexOf('── Other lenses ──')).toBeLessThan(labels.indexOf('Nikon Other'));
	});

	it('treats all large-format mounts as cross-compatible when the camera is LF', () => {
		const mounts = [mount(10, 'Copal #0'), mount(11, 'Compur #1')];
		const lenses = [
			lens({ id: 1, model: 'Copal lens', lens_mount_id: 10 }),
			lens({ id: 2, model: 'Compur lens', lens_mount_id: 11 })
		];
		const opts = buildLensOptions(lenses, { lens_mount_id: 10 }, 'No default lens', mounts);
		// Both LF lenses are compatible → no "Other lenses" divider is emitted.
		expect(opts.some((o) => o.label === '── Other lenses ──')).toBe(false);
		expect(opts.filter((o) => o.value && !o.disabled)).toHaveLength(2);
	});

	it('lists every owned lens (no divider) when no camera is selected', () => {
		const opts = buildLensOptions([lens({ id: 1, model: 'A' }), lens({ id: 2, model: 'B' })], null);
		expect(opts.some((o) => o.disabled)).toBe(false);
		expect(opts[0]).toEqual({ value: '', label: 'No default lens' });
	});
});
