// Standard aperture / shutter values for the guided shot-entry inputs.
// Suggestion lists (dropdown) are intentionally concise; the "recognized" sets
// behind the off-list ⚠ hint are broad so only genuine typos are flagged.
// All values are BARE — aperture "5.6" (never "f/5.6"), shutter "1/250"
// (never "1/250s") — because every display site prepends f/ / appends s.

/** Half-stop apertures — the f/ dropdown suggestions. */
export const APERTURE_SUGGESTIONS = [
	'1', '1.2', '1.4', '1.7', '2', '2.4', '2.8', '3.4', '4', '4.8',
	'5.6', '6.7', '8', '9.5', '11', '13', '16', '19', '22', '27', '32'
];

/** Standard shutter speeds, fast → slow, then whole seconds, then bulb. */
export const SHUTTER_SUGGESTIONS = [
	'1/4000', '1/2000', '1/1000', '1/500', '1/250', '1/125', '1/60', '1/30',
	'1/15', '1/8', '1/4', '1/2', '1', '2', '4', '8', '15', '30', 'B'
];

/** Broad recognized aperture set (full ∪ half ∪ third stops, f/0.95–f/64). */
const RECOGNIZED_APERTURES = new Set([
	'0.95', '1', '1.1', '1.2', '1.4', '1.6', '1.7', '1.8', '2', '2.2', '2.4', '2.5',
	'2.8', '3.2', '3.4', '3.5', '4', '4.5', '4.8', '5', '5.6', '6.3', '6.7', '7.1',
	'8', '9', '9.5', '10', '11', '13', '14', '16', '18', '19', '20', '22', '25', '27',
	'29', '32', '36', '40', '45', '51', '57', '64'
]);

/** Broad recognized shutter set (standard + common legacy/leaf speeds + long). */
const RECOGNIZED_SHUTTERS = new Set([
	'1/8000', '1/4000', '1/2000', '1/1000', '1/500', '1/400', '1/320', '1/300',
	'1/250', '1/200', '1/160', '1/125', '1/100', '1/90', '1/80', '1/60', '1/50',
	'1/45', '1/40', '1/30', '1/25', '1/20', '1/15', '1/10', '1/8', '1/5', '1/4',
	'1/2', '1', '2', '4', '8', '15', '30', '60', 'B', 'T'
]);

/** Strip a leading f//f, comma→dot, remove whitespace. Emits a bare aperture. */
export function normalizeAperture(v: string): string {
	return v.replace(/\s+/g, '').replace(/^f\/?/i, '').replace(',', '.');
}

/** Strip a trailing s/sec/seconds and whitespace. Emits a bare shutter value. */
export function normalizeShutter(v: string): string {
	return v.replace(/\s+/g, '').replace(/(s|sec|secs|second|seconds)$/i, '');
}

/** True if the (normalized) value is a recognized standard aperture. */
export function isRecognizedAperture(v: string): boolean {
	const n = normalizeAperture(v);
	return n !== '' && RECOGNIZED_APERTURES.has(n);
}

/** True if the (normalized) value is a recognized standard shutter speed. */
export function isRecognizedShutter(v: string): boolean {
	const n = normalizeShutter(v);
	return n !== '' && RECOGNIZED_SHUTTERS.has(n);
}
