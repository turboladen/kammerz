import { describe, it, expect } from 'vitest';
import { buildFrameCells, nextExtraFrameNumber, DEFAULT_FRAMES } from './frames';
import type { Shot } from '$lib/types';

function shot(frame_number: string, over: Partial<Shot> = {}): Shot {
	return {
		id: 1,
		roll_id: 1,
		frame_number,
		aperture: null,
		shutter_speed: null,
		date: null,
		date_fuzzy: null,
		location: null,
		gps_lat: null,
		gps_lon: null,
		notes: null,
		created_at: '',
		updated_at: '',
		...over
	};
}

describe('buildFrameCells', () => {
	it('defaults to 36 frames when frameCount is null', () => {
		const cells = buildFrameCells([], null);
		expect(cells).toHaveLength(DEFAULT_FRAMES);
		expect(cells[0]).toEqual({ frameNumber: '1', shot: null, isNext: true });
		expect(cells[1].isNext).toBe(false);
	});

	it('marks the first empty slot as next with shots filled in order', () => {
		const cells = buildFrameCells([shot('1'), shot('2')], 24);
		expect(cells).toHaveLength(24);
		expect(cells[0].shot?.frame_number).toBe('1');
		expect(cells[1].shot?.frame_number).toBe('2');
		expect(cells[2]).toMatchObject({ frameNumber: '3', isNext: true });
	});

	it('has no next slot when the roll is full', () => {
		const shots = Array.from({ length: 3 }, (_, i) => shot(String(i + 1)));
		const cells = buildFrameCells(shots, 3);
		expect(cells).toHaveLength(3);
		expect(cells.some((c) => c.isNext)).toBe(false);
	});

	it('appends extras (frame numbers outside 1..n) after the numbered slots', () => {
		const cells = buildFrameCells([shot('1'), shot('37')], 36);
		expect(cells).toHaveLength(37);
		expect(cells[36]).toMatchObject({ frameNumber: '37', isNext: false });
		expect(cells[36].shot?.frame_number).toBe('37');
	});

	it('treats whitespace-padded frame numbers as their trimmed slot', () => {
		const cells = buildFrameCells([shot(' 2 ')], 4);
		expect(cells[1].shot?.frame_number).toBe(' 2 ');
		expect(cells[0].isNext).toBe(true); // slot 1 is the first open one
	});
});

describe('nextExtraFrameNumber', () => {
	it('returns frameCount + 1 when no shots exceed the count', () => {
		expect(nextExtraFrameNumber([shot('1')], 36)).toBe('37');
	});

	it('returns one past the highest numeric frame when extras exist', () => {
		expect(nextExtraFrameNumber([shot('36'), shot('37')], 36)).toBe('38');
	});

	it('defaults the base to 36 when frameCount is null', () => {
		expect(nextExtraFrameNumber([], null)).toBe('37');
	});

	it('ignores non-numeric frame numbers', () => {
		expect(nextExtraFrameNumber([shot('36A')], 36)).toBe('37');
	});
});
