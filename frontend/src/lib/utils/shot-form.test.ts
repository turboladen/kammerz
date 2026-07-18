import { describe, expect, it } from 'vitest';
import { buildShotUpdatePayload, shotFormsEqual, type ShotFormFields } from './shot-form';

const base: ShotFormFields = {
	frameNumber: '9',
	aperture: '5.6',
	shutterSpeed: '1/250',
	date: '2024-06-16',
	time: '12:30',
	location: 'Corlieu Falls',
	notes: 'first light',
	lensId: '3'
};

describe('shotFormsEqual', () => {
	it('is true for identical field sets', () => {
		expect(shotFormsEqual(base, { ...base })).toBe(true);
	});

	it('is false when any single field differs', () => {
		for (const key of Object.keys(base) as (keyof ShotFormFields)[]) {
			expect(shotFormsEqual(base, { ...base, [key]: base[key] + 'x' })).toBe(false);
		}
	});
});

describe('buildShotUpdatePayload', () => {
	it('maps populated fields, normalizing exposure values to bare form', () => {
		expect(buildShotUpdatePayload({ ...base, frameNumber: ' 9 ', aperture: 'f/2.8', shutterSpeed: '1/250s' })).toEqual({
			frame_number: '9',
			aperture: '2.8',
			shutter_speed: '1/250',
			date: '2024-06-16',
			time: '12:30',
			location: 'Corlieu Falls',
			notes: 'first light',
			lens_ids: [3]
		});
	});

	it('collapses empty optionals to null and empty lens to []', () => {
		expect(
			buildShotUpdatePayload({
				frameNumber: '1',
				aperture: '',
				shutterSpeed: '',
				date: '',
				time: '   ',
				location: '',
				notes: '',
				lensId: ''
			})
		).toEqual({
			frame_number: '1',
			aperture: null,
			shutter_speed: null,
			date: null,
			time: null,
			location: null,
			notes: null,
			lens_ids: []
		});
	});

	it('canonicalizes a valid compact time and passes an invalid time through raw', () => {
		expect(buildShotUpdatePayload({ ...base, time: '1430' }).time).toBe('14:30');
		expect(buildShotUpdatePayload({ ...base, time: '99:99' }).time).toBe('99:99');
	});
});
