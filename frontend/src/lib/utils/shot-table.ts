// Pure display helpers for the shots table + view-first dialog (kammerz-4she).
//
// Aperture and shutter are stored BARE (frontend-patterns.md): aperture "2.8",
// shutter "1/125" or "4". Every display site prepends `f/` / appends `s` — these
// helpers centralize that convention so the table, the read-only view dialog, and
// their unit tests can't drift.
import type { Lens, Shot } from '$lib/types';
import { lensDisplayName } from './lens';

/** A shot's fields rendered for display — bare exposure values already decorated. */
export interface ShotRowDisplay {
	frame: string;
	aperture: string;
	shutter: string;
	date: string;
	time: string;
	location: string;
	notes: string;
}

/**
 * Map a shot onto its display strings: `f/`-prefixed aperture, `s`-suffixed
 * shutter, and plain text for the rest. A null field renders as an empty string
 * (never a bare "f/" or "s"), so the caller decides how to show absence.
 */
// Keep this LOSSLESS: the table cells, the read-only view dialog, the FrameStrip
// hint strings, and the last-shot reference card all render from it. Compaction
// (truncating notes, abbreviating location) belongs at the render site that
// needs it — baked in here it would silently degrade every other consumer.
export function formatShotRow(shot: Shot): ShotRowDisplay {
	return {
		frame: shot.frame_number,
		aperture: shot.aperture ? `f/${shot.aperture}` : '',
		shutter: shot.shutter_speed ? `${shot.shutter_speed}s` : '',
		date: shot.date ?? '',
		time: shot.time ?? '',
		location: shot.location ?? '',
		notes: shot.notes ?? ''
	};
}

/** A shot's effective lens display, plus whether it was inherited from the roll. */
export interface ShotLensDisplay {
	name: string;
	/** True when `name` came from the roll-default fallback, not the shot's own ids. */
	inherited: boolean;
}

/**
 * The effective lens display for a shot: the shot's own per-shot lens ids (joined
 * with ", ") take priority; otherwise the roll's default lens; otherwise empty.
 * Ids absent from `lenses` are skipped — and unlike the print page's
 * `shotLensDisplay`, an all-unresolvable id list falls through to the roll
 * default rather than returning '' (more helpful; near-impossible under FKs).
 * `inherited` is decided HERE, alongside the name, so no consumer ever
 * re-derives own-vs-default from the raw map and drifts from this priority.
 */
export function resolveShotLensName(
	shotId: number,
	shotLensMap: Record<number, number[]>,
	lenses: Lens[],
	fallbackLensId: number | null
): ShotLensDisplay {
	const ids = shotLensMap[shotId] ?? [];
	if (ids.length > 0) {
		const names = ids
			.map((lid) => lenses.find((l) => l.id === lid))
			.filter((l): l is Lens => l != null)
			.map((l) => lensDisplayName(l));
		if (names.length > 0) return { name: names.join(', '), inherited: false };
	}
	if (fallbackLensId != null) {
		const def = lenses.find((l) => l.id === fallbackLensId);
		if (def) return { name: lensDisplayName(def), inherited: true };
	}
	return { name: '', inherited: false };
}
