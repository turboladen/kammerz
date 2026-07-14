import { createShot, suggestNextFrame } from '$lib/api/shots';
import { parseTime } from '$lib/utils/time';

export interface QuickShotInput {
	rollId: number;
	frameNumber: string;
	aperture?: string;
	shutterSpeed?: string;
	lensId?: string; // '' = none
	date?: string; // '' = none
	time?: string; // '' = none
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
	// Canonicalize 24h time (e.g. "1430" → "14:30") so a keyboard Save (⌘/Ctrl+Enter)
	// that fires before the TimeInput's blur-normalize still sends a value the backend's
	// strict `validate_time` accepts. A non-empty unparseable value is passed through so
	// the 422 surfaces it instead of silently dropping the shot's time.
	const rawTime = input.time?.trim() || '';
	await createShot({
		roll_id: input.rollId,
		frame_number: input.frameNumber.trim(),
		aperture: input.aperture?.trim() || null,
		shutter_speed: input.shutterSpeed?.trim() || null,
		date: input.date?.trim() || null,
		time: parseTime(rawTime) || rawTime || null,
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
