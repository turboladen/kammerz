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

	it('prepends sub-1 extras (e.g. the leader frame 0) before slot 1, ascending', () => {
		// Roll M67-4 repro: frames 0, 3, 10 must show 0 first, not after 3.
		const cells = buildFrameCells([shot('0'), shot('3'), shot('10')], 11);
		expect(cells).toHaveLength(12); // 1 leader + 11 numbered slots
		expect(cells[0]).toMatchObject({ frameNumber: '0', isNext: false });
		expect(cells[0].shot?.frame_number).toBe('0');
		expect(cells[1].frameNumber).toBe('1');
		// '0' precedes both numbered shots
		const idx = (fn: string) => cells.findIndex((c) => c.shot?.frame_number === fn);
		expect(idx('0')).toBeLessThan(idx('3'));
		expect(idx('0')).toBeLessThan(idx('10'));
	});

	it('orders multiple sub-1 extras ascending (negatives before 0)', () => {
		const cells = buildFrameCells([shot('0'), shot('-1')], 36);
		expect(cells[0].frameNumber).toBe('-1');
		expect(cells[1].frameNumber).toBe('0');
		expect(cells[2].frameNumber).toBe('1'); // numbered slots start after the prepended extras
	});

	it('orders over-roll extras ascending after the last numbered slot', () => {
		// 12 and 37 both over an 11-frame roll — ascending, not insertion order (37 added first).
		const cells = buildFrameCells([shot('1'), shot('37'), shot('12')], 11);
		expect(cells).toHaveLength(13); // 11 slots + 2 over-roll
		expect(cells[11].frameNumber).toBe('12');
		expect(cells[12].frameNumber).toBe('37');
	});

	it('keeps non-numeric extras trailing at the very end without crashing', () => {
		const cells = buildFrameCells([shot('36A'), shot('1')], 36);
		expect(cells).toHaveLength(37);
		expect(cells[36]).toMatchObject({ frameNumber: '36A', isNext: false });
		expect(cells[36].shot?.frame_number).toBe('36A');
	});

	it('orders a mixed batch: sub-1 first, slots, over-roll ascending, then non-numeric', () => {
		const cells = buildFrameCells([shot('0'), shot('36A'), shot('37'), shot('1')], 36);
		expect(cells).toHaveLength(39); // 1 leader + 36 slots + 1 over-roll + 1 non-numeric
		expect(cells[0].frameNumber).toBe('0');
		expect(cells[1].frameNumber).toBe('1');
		expect(cells[37].frameNumber).toBe('37');
		expect(cells[38].frameNumber).toBe('36A');
	});

	it('trails an in-range but unslotted numeric (e.g. a half-frame 1.5) at the very end', () => {
		// 1.5 parses in [1, n] but matches no integer slot → trailing, not prepended/over-roll.
		const cells = buildFrameCells([shot('1.5'), shot('1')], 36);
		expect(cells).toHaveLength(37); // 36 slots + 1 trailing
		expect(cells[36]).toMatchObject({ frameNumber: '1.5', isNext: false });
	});

	it('prepends duplicate-valued sub-1 extras (0 and 00) before slot 1, stable by insertion order', () => {
		const cells = buildFrameCells([shot('00'), shot('0')], 36);
		// Both parse to 0; the stable sort keeps them in insertion order ('00' was added first).
		expect(cells[0].frameNumber).toBe('00');
		expect(cells[1].frameNumber).toBe('0');
		expect(cells[2].frameNumber).toBe('1');
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
