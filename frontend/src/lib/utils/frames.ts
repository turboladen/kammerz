import type { Shot } from '$lib/types';

export interface FrameCell {
	frameNumber: string;
	shot: Shot | null;
	isNext: boolean;
}

/** Default frame count assumed when a roll's frame_count is null. */
export const DEFAULT_FRAMES = 36;

/**
 * Parse a frame number for ordering/classification. Uses `Number()` (not `parseInt`) so a
 * partly-numeric string like "36A" is treated as non-numeric (null) rather than 36 —
 * `parseInt("36A")` would wrongly yield 36. Returns null for blank, NaN, or ±Infinity so
 * only genuine finite numbers get repositioned; everything else stays put (trailing).
 */
function parseFrameNumber(fn: string): number | null {
	const t = fn.trim();
	if (t === '') return null;
	const num = Number(t);
	return Number.isFinite(num) ? num : null;
}

/**
 * Map a roll's shots onto numbered frame slots `1..n` (n = frameCount, or
 * DEFAULT_FRAMES when null). The first empty slot is flagged `isNext`. Shots whose frame
 * number falls outside `1..n` are ordered so the strip reads left-to-right: sub-1 leaders
 * (e.g. "0", "00") are PREPENDED before slot 1 ascending, over-roll frames (e.g. "37") are
 * appended after slot n ascending, and unparseable / in-range-but-unslotted frames (e.g.
 * "36A", "01") trail at the very end in insertion order (kammerz-m7a). Extras are never
 * flagged next. Shared by the roll-detail page and the Quick Entry page so the film strip
 * behaves identically in both.
 */
export function buildFrameCells(shots: Shot[], frameCount: number | null): FrameCell[] {
	const n = frameCount ?? DEFAULT_FRAMES;
	const byFrame = new Map<string, Shot>();
	for (const s of shots) byFrame.set(s.frame_number.trim(), s);

	const slots: FrameCell[] = [];
	let nextAssigned = false;
	for (let i = 1; i <= n; i++) {
		const fn = String(i);
		const shot = byFrame.get(fn) ?? null;
		const isNext = !shot && !nextAssigned;
		if (isNext) nextAssigned = true;
		slots.push({ frameNumber: fn, shot, isNext });
		byFrame.delete(fn);
	}

	// Extras: shots whose frame_number wasn't a 1..n slot. Partition by parsed value so the
	// strip stays in reading order instead of dumping everything after slot n.
	const below: FrameCell[] = []; // numeric < 1 → before slot 1
	const over: FrameCell[] = []; // numeric > n → after slot n
	const trailing: FrameCell[] = []; // non-numeric, or in-range unslotted numerics → end
	for (const [fn, shot] of byFrame) {
		const num = parseFrameNumber(fn);
		const cell: FrameCell = { frameNumber: fn, shot, isNext: false };
		if (num !== null && num < 1) below.push(cell);
		else if (num !== null && num > n) over.push(cell);
		else trailing.push(cell);
	}
	// `below`/`over` are all-numeric by construction, so the parse is non-null here; sort is
	// stable (ES2019+) so equal-valued strings (e.g. "0" and "00") keep insertion order.
	const byValue = (a: FrameCell, b: FrameCell) => parseFrameNumber(a.frameNumber)! - parseFrameNumber(b.frameNumber)!;
	below.sort(byValue);
	over.sort(byValue);

	return [...below, ...slots, ...over, ...trailing];
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
