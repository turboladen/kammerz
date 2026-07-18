import { normalizeAperture, normalizeShutter } from '$lib/utils/exposure';
import { parseTime } from '$lib/utils/time';

/**
 * The Edit/Add Shot dialog's form fields as plain strings — the shape the
 * roll-detail page holds in $state. Pure so both the dirty-compare and the
 * update payload can be unit tested away from the component (kammerz-11o3).
 */
export interface ShotFormFields {
	frameNumber: string;
	aperture: string;
	shutterSpeed: string;
	date: string;
	time: string;
	location: string;
	notes: string;
	lensId: string;
}

/** Field-by-field equality — used to decide whether navigation must save first. */
export function shotFormsEqual(a: ShotFormFields, b: ShotFormFields): boolean {
	return (
		a.frameNumber === b.frameNumber &&
		a.aperture === b.aperture &&
		a.shutterSpeed === b.shutterSpeed &&
		a.date === b.date &&
		a.time === b.time &&
		a.location === b.location &&
		a.notes === b.notes &&
		a.lensId === b.lensId
	);
}

/**
 * Build the PUT /api/shots/{id} body from the form fields. Exposure values are
 * normalized to their bare stored form; a valid time canonicalizes to HH:MM,
 * blank collapses to null, and an invalid time passes through raw so the
 * backend's 422 surfaces the mistake instead of silently dropping it.
 */
export function buildShotUpdatePayload(f: ShotFormFields) {
	return {
		frame_number: f.frameNumber.trim(),
		aperture: normalizeAperture(f.aperture) || null,
		shutter_speed: normalizeShutter(f.shutterSpeed) || null,
		date: f.date || null,
		time: parseTime(f.time) || f.time.trim() || null,
		location: f.location || null,
		notes: f.notes || null,
		lens_ids: f.lensId ? [Number(f.lensId)] : []
	};
}
