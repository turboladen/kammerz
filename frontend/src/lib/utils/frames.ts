import type { Shot } from '$lib/types';

export interface FrameCell {
	frameNumber: string;
	shot: Shot | null;
	isNext: boolean;
}

/** Default frame count assumed when a roll's frame_count is null. */
export const DEFAULT_FRAMES = 36;

/**
 * Map a roll's shots onto numbered frame slots `1..n` (n = frameCount, or
 * DEFAULT_FRAMES when null). The first empty slot is flagged `isNext`. Shots whose
 * frame number falls outside `1..n` (e.g. "37", "00", "36A" over-rolls) are appended
 * after the numbered slots, never flagged next. Shared by the roll-detail page and the
 * Quick Entry page so the film strip behaves identically in both.
 */
export function buildFrameCells(shots: Shot[], frameCount: number | null): FrameCell[] {
	const n = frameCount ?? DEFAULT_FRAMES;
	const byFrame = new Map<string, Shot>();
	for (const s of shots) byFrame.set(s.frame_number.trim(), s);

	const cells: FrameCell[] = [];
	let nextAssigned = false;
	for (let i = 1; i <= n; i++) {
		const fn = String(i);
		const shot = byFrame.get(fn) ?? null;
		const isNext = !shot && !nextAssigned;
		if (isNext) nextAssigned = true;
		cells.push({ frameNumber: fn, shot, isNext });
		byFrame.delete(fn);
	}
	// Extras: any shot whose frame_number wasn't a 1..n slot (e.g. "37", "00", "36A").
	for (const [fn, shot] of byFrame) cells.push({ frameNumber: fn, shot, isNext: false });
	return cells;
}

/**
 * Frame number for the next over-roll / extra frame: one past whichever is larger of
 * the roll's frame count and the highest numeric frame already logged. Used by the
 * Quick Entry "+" button to target a frame the sequential next-slot logic can't reach
 * (e.g. a 37th frame on a 36-exposure roll). Non-numeric frame numbers are ignored.
 */
export function nextExtraFrameNumber(shots: Shot[], frameCount: number | null): string {
	let max = frameCount ?? DEFAULT_FRAMES;
	for (const s of shots) {
		const num = parseInt(s.frame_number.trim(), 10);
		if (!Number.isNaN(num) && num > max) max = num;
	}
	return String(max + 1);
}
