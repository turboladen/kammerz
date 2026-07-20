import { describe, expect, it } from 'vitest';
import { formatShotRow, resolveShotLensName } from './shot-table';
import type { Lens, Shot } from '$lib/types';

function shot(over: Partial<Shot> = {}): Shot {
	return {
		id: 1,
		roll_id: 1,
		frame_number: '1',
		aperture: null,
		shutter_speed: null,
		date: null,
		time: null,
		location: null,
		gps_lat: null,
		gps_lon: null,
		notes: null,
		created_at: '',
		updated_at: '',
		...over
	};
}

function lens(over: Partial<Lens> = {}): Lens {
	return {
		id: 1,
		brand: 'Nikon',
		lens_mount_id: 1,
		lens_system: null,
		model: null,
		focal_length: '50',
		max_aperture: '1.8',
		min_aperture: null,
		filter_thread_front_mm: null,
		filter_thread_rear_mm: null,
		serial_number: null,
		date_purchased: null,
		purchased_from: null,
		date_sold: null,
		notes: null,
		created_at: '',
		updated_at: '',
		...over
	};
}

describe('formatShotRow', () => {
	it('prepends f/ to aperture and appends s to shutter (stored bare)', () => {
		expect(
			formatShotRow(
				shot({
					frame_number: '12',
					aperture: '5.6',
					shutter_speed: '1/250',
					date: '2024-06-16',
					time: '14:30',
					location: 'Corlieu Falls',
					notes: 'first light'
				})
			)
		).toEqual({
			frame: '12',
			aperture: 'f/5.6',
			shutter: '1/250s',
			date: '2024-06-16',
			time: '14:30',
			location: 'Corlieu Falls',
			notes: 'first light'
		});
	});

	it('renders empty strings for null fields (never "f/" or "s" alone)', () => {
		expect(formatShotRow(shot({ frame_number: '3' }))).toEqual({
			frame: '3',
			aperture: '',
			shutter: '',
			date: '',
			time: '',
			location: '',
			notes: ''
		});
	});

	it('formats aperture and shutter independently of each other', () => {
		expect(formatShotRow(shot({ aperture: '8' })).shutter).toBe('');
		expect(formatShotRow(shot({ aperture: '8' })).aperture).toBe('f/8');
		expect(formatShotRow(shot({ shutter_speed: '4' })).aperture).toBe('');
		expect(formatShotRow(shot({ shutter_speed: '4' })).shutter).toBe('4s');
	});
});

describe('resolveShotLensName', () => {
	const lenses = [
		lens({ id: 3, brand: 'Nikon', model: 'AI-S 50mm' }),
		lens({ id: 7, brand: 'Canon', model: 'FD 35mm' })
	];

	it("uses the shot's own lens when present", () => {
		expect(resolveShotLensName(1, { 1: [3] }, lenses, 7)).toEqual({ name: 'Nikon AI-S 50mm', inherited: false });
	});

	it('joins multiple lens names with a comma', () => {
		expect(resolveShotLensName(1, { 1: [3, 7] }, lenses, null)).toEqual({
			name: 'Nikon AI-S 50mm, Canon FD 35mm',
			inherited: false
		});
	});

	it('falls back to the roll default lens when the shot has none', () => {
		expect(resolveShotLensName(1, {}, lenses, 7)).toEqual({ name: 'Canon FD 35mm', inherited: true });
		expect(resolveShotLensName(1, { 1: [] }, lenses, 3)).toEqual({ name: 'Nikon AI-S 50mm', inherited: true });
	});

	it('returns empty when neither the shot nor the roll has a resolvable lens', () => {
		expect(resolveShotLensName(1, {}, lenses, null)).toEqual({ name: '', inherited: false });
		// A lens id not in the catalog resolves to nothing, not a crash.
		expect(resolveShotLensName(1, { 1: [999] }, lenses, null)).toEqual({ name: '', inherited: false });
		// All-unresolvable own ids fall through to the roll default AND are marked
		// inherited — the flag comes from the same function as the name, so the
		// view's '(roll default)' annotation can't disagree with the resolution.
		expect(resolveShotLensName(1, { 1: [999] }, lenses, 7)).toEqual({ name: 'Canon FD 35mm', inherited: true });
		expect(resolveShotLensName(1, {}, lenses, 999)).toEqual({ name: '', inherited: false });
	});
});
