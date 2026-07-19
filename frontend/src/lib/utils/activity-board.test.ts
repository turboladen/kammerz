import { describe, it, expect } from 'vitest';
import {
	rollPhase,
	activityLabel,
	stateLabel,
	ROLL_DATE_FIELD,
	SLOT_CAPTIONS,
	isDatedKind,
	lastShotSummary
} from './activity-board';
import type { Lens, Shot } from '$lib/types';

function shot(overrides: Partial<Shot>): Shot {
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
		...overrides
	};
}

function lens(overrides: Partial<Lens>): Lens {
	return {
		id: 1,
		brand: 'Nikon',
		lens_mount_id: 1,
		lens_system: null,
		model: '50mm f/1.8',
		focal_length: '50mm',
		max_aperture: 'f/1.8',
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
		...overrides
	};
}

describe('rollPhase', () => {
	it('returns done whenever the roll is done, regardless of group_key', () => {
		expect(rollPhase({ done: true, group_key: 5 })).toBe('done');
		// done wins even if group_key looks mid-flow (defensive — backend keeps them consistent)
		expect(rollPhase({ done: true, group_key: 0 })).toBe('done');
	});

	it('returns shooting only when shooting is the earliest unresolved activity', () => {
		expect(rollPhase({ done: false, group_key: 0 })).toBe('shooting');
	});

	it('returns wrapup for every post-shooting group_key', () => {
		for (const group_key of [1, 2, 3, 4]) {
			expect(rollPhase({ done: false, group_key })).toBe('wrapup');
		}
	});
});

describe('activityLabel', () => {
	it('labels every activity kind', () => {
		expect(activityLabel('shooting')).toBe('Shooting');
		expect(activityLabel('development')).toBe('Development');
		expect(activityLabel('scanning')).toBe('Scanning');
		expect(activityLabel('post_processing')).toBe('Post-processing');
		expect(activityLabel('archiving')).toBe('Archiving');
	});
});

describe('stateLabel', () => {
	it('labels the shared states', () => {
		expect(stateLabel('shooting', 'in_progress')).toBe('In progress');
		expect(stateLabel('scanning', 'done')).toBe('Done');
		expect(stateLabel('archiving', 'na')).toBe('N/A');
	});

	it('reads not_started per activity — archiving says "Not archived"', () => {
		expect(stateLabel('shooting', 'not_started')).toBe('Not started');
		expect(stateLabel('scanning', 'not_started')).toBe('Not started');
		expect(stateLabel('archiving', 'not_started')).toBe('Not archived');
	});
});

describe('ROLL_DATE_FIELD', () => {
	it('maps each roll-owned slot to its writable column', () => {
		expect(ROLL_DATE_FIELD.shooting).toEqual({ start: 'date_loaded', completion: 'date_finished' });
		expect(ROLL_DATE_FIELD.scanning).toEqual({ start: 'scan_started', completion: 'date_scanned' });
		expect(ROLL_DATE_FIELD.post_processing).toEqual({
			start: 'post_processing_started',
			completion: 'date_post_processed'
		});
		expect(ROLL_DATE_FIELD.archiving).toEqual({ completion: 'date_archived' });
	});

	it('never maps development (its dates live on the dev record, edited via the dev dialog)', () => {
		expect(ROLL_DATE_FIELD.development).toBeUndefined();
	});
});

describe('lastShotSummary', () => {
	it('returns null when there are no shots', () => {
		expect(lastShotSummary([], {}, [])).toBeNull();
	});

	it('summarizes the highest-frame (most recent) shot with its lens name', () => {
		const shots = [
			shot({ id: 10, frame_number: '1', aperture: '8', shutter_speed: '1/250' }),
			shot({ id: 11, frame_number: '2', aperture: '5.6', shutter_speed: '1/125' })
		];
		const summary = lastShotSummary(shots, { 11: [7] }, [lens({ id: 7, brand: 'Leica', model: 'Summicron' })]);
		expect(summary).toEqual({
			frame: '2',
			aperture: '5.6',
			shutter: '1/125',
			lensName: 'Leica Summicron'
		});
	});

	it('leaves lensName null when the shot has no lens', () => {
		const shots = [shot({ id: 10, frame_number: '3' })];
		expect(lastShotSummary(shots, {}, [lens({ id: 7 })])?.lensName).toBeNull();
	});

	it('leaves lensName null when the mapped lens id is not in the catalog', () => {
		const shots = [shot({ id: 10, frame_number: '3' })];
		expect(lastShotSummary(shots, { 10: [999] }, [lens({ id: 7 })])?.lensName).toBeNull();
	});
});

describe('SLOT_CAPTIONS / isDatedKind', () => {
	it('classifies exactly the roll-owned dated activities', () => {
		expect(isDatedKind('shooting')).toBe(true);
		expect(isDatedKind('scanning')).toBe(true);
		expect(isDatedKind('post_processing')).toBe(true);
		expect(isDatedKind('development')).toBe(false);
		expect(isDatedKind('archiving')).toBe(false);
	});

	it('captions every slot of every dated activity, mirroring ROLL_DATE_FIELD', () => {
		expect(SLOT_CAPTIONS.shooting).toEqual({ start: 'Loaded', completion: 'Finished' });
		expect(SLOT_CAPTIONS.scanning).toEqual({ start: 'Started', completion: 'Scanned' });
		expect(SLOT_CAPTIONS.post_processing).toEqual({ start: 'Started', completion: 'Done' });
		// Every dated kind with a caption must have matching writable columns and
		// vice versa (archiving is date-mapped but dialog-edited, so not captioned).
		for (const kind of ['shooting', 'scanning', 'post_processing'] as const) {
			expect(Object.keys(SLOT_CAPTIONS[kind]).sort()).toEqual(Object.keys(ROLL_DATE_FIELD[kind] ?? {}).sort());
		}
	});
});
