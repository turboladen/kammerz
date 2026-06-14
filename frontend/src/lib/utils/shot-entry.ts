import { createShot, suggestNextFrame } from '$lib/api/shots';

export interface QuickShotInput {
	rollId: number;
	frameNumber: string;
	aperture?: string;
	shutterSpeed?: string;
	lensId?: string; // '' = none
	date?: string; // '' = none
	location?: string;
	notes?: string;
}

/**
 * Create one shot and return the suggested next frame number. Shared by the
 * Quick Entry page and the inline QuickAddBar so save-and-advance behaves
 * identically. Throws on API error (caller shows the message). `date`/`location`
 * default to none — the Quick Entry page passes blanks; the QuickAddBar passes
 * them only when the "more" fields are filled.
 */
export async function logShot(input: QuickShotInput): Promise<string> {
	const lensIds = input.lensId ? [Number(input.lensId)] : [];
	await createShot({
		roll_id: input.rollId,
		frame_number: input.frameNumber.trim(),
		aperture: input.aperture?.trim() || null,
		shutter_speed: input.shutterSpeed?.trim() || null,
		date: input.date?.trim() || null,
		date_fuzzy: null,
		location: input.location?.trim() || null,
		gps_lat: null,
		gps_lon: null,
		notes: input.notes?.trim() || null,
		lens_ids: lensIds
	});
	try {
		return await suggestNextFrame(input.rollId);
	} catch {
		return '';
	}
}
