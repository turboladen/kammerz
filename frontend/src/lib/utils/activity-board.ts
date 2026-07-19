// Pure logic for the roll activity board + adaptive phase layout (ADR-0013).
//
// The backend derives every lifecycle value from date presence and returns it on
// the roll (`activities`/`badge`/`group_key`/`done`). This module never re-derives
// state from dates — it maps those server-computed values onto display labels, the
// page phase, and the writable roll columns each board row edits.
import type { ActivityKind, ActivityState, Lens, RollActivityView, Shot } from '$lib/types';
import { lensDisplayName } from './lens';

/** The page's layout phase, derived purely from the server's group_key/done. */
export type RollPhase = 'shooting' | 'wrapup' | 'done';

/**
 * Which layout the roll page renders. Derived ONLY from the backend's `done` /
 * `group_key` (never re-derived from dates): a done roll is `done`; otherwise
 * `shooting` while shooting is the earliest unresolved activity (group_key 0), and
 * `wrapup` for every post-shooting state (group_key 1..4 — shot, at-lab/developing,
 * developed, scanned, post-processed). Wrap-up hides quick entry by default.
 */
export function rollPhase(v: Pick<RollActivityView, 'done' | 'group_key'>): RollPhase {
	if (v.done) return 'done';
	return v.group_key === 0 ? 'shooting' : 'wrapup';
}

/** Human label for an activity row heading. */
export function activityLabel(kind: ActivityKind): string {
	switch (kind) {
		case 'shooting':
			return 'Shooting';
		case 'development':
			return 'Development';
		case 'scanning':
			return 'Scanning';
		case 'post_processing':
			return 'Post-processing';
		case 'archiving':
			return 'Archiving';
	}
}

/**
 * Human label for an activity's derived state. Archiving is a moment, not a
 * duration, so its unstarted state reads "Not archived" rather than "Not started".
 */
export function stateLabel(kind: ActivityKind, state: ActivityState): string {
	switch (state) {
		case 'in_progress':
			return 'In progress';
		case 'done':
			return 'Done';
		case 'na':
			return 'N/A';
		case 'not_started':
			return kind === 'archiving' ? 'Not archived' : 'Not started';
	}
}

/** A writable date column on the roll row. */
export type RollDateField =
	| 'date_loaded'
	| 'date_finished'
	| 'scan_started'
	| 'date_scanned'
	| 'post_processing_started'
	| 'date_post_processed'
	| 'date_archived';

/** A board date slot within a row. */
export type DateSlot = 'start' | 'completion';

/** The activities whose start/completion dates live on the roll row itself. */
export type DatedActivityKind = 'shooting' | 'scanning' | 'post_processing';

/**
 * Per-slot captions for the dated activities — the single source for both the
 * board's row captions and the accessible names / dialog titles built from them,
 * so a slot can never render under one label and announce another.
 */
export const SLOT_CAPTIONS: Record<DatedActivityKind, Record<DateSlot, string>> = {
	shooting: { start: 'Loaded', completion: 'Finished' },
	scanning: { start: 'Started', completion: 'Scanned' },
	post_processing: { start: 'Started', completion: 'Done' }
};

/** Narrow an activity kind to the dated (roll-row date slots) subset. */
export function isDatedKind(kind: ActivityKind): kind is DatedActivityKind {
	return kind in SLOT_CAPTIONS;
}

/**
 * The one human phrase naming a dated slot — "Shooting finished date",
 * "Scanning started date". Single source for the board's accessible names, the
 * DateConfirm titles, and the clear-confirmation labels, so the name a control
 * announces and the name its dialogs display can never drift apart.
 */
export function slotDateLabel(kind: DatedActivityKind, slot: DateSlot): string {
	return `${activityLabel(kind)} ${SLOT_CAPTIONS[kind][slot].toLowerCase()} date`;
}

/**
 * Which roll column each activity's start/completion date writes to. Development is
 * intentionally absent — its dates live on the lab/self dev record and are edited
 * through the development dialog, never the board. Archiving is a moment, so it has
 * only a completion column and no start — and its entry is DECLARATIVE only (it
 * completes the map + its test): at runtime archiving edits go through the
 * ArchiveDialog's compound payload (date + location + N/A + reason), never through
 * onEditDate/onClearDate, which only the dated kinds reach.
 */
export const ROLL_DATE_FIELD: Partial<Record<ActivityKind, Partial<Record<DateSlot, RollDateField>>>> = {
	shooting: { start: 'date_loaded', completion: 'date_finished' },
	scanning: { start: 'scan_started', completion: 'date_scanned' },
	post_processing: { start: 'post_processing_started', completion: 'date_post_processed' },
	archiving: { completion: 'date_archived' }
};

/** The archive fields a single ArchiveDialog save writes to the roll. */
export interface ArchivePayload {
	date_archived: string | null;
	archive_location: string | null;
	archive_na: boolean;
	archive_na_reason: string | null;
}

/** The "what settings did I just use?" reference card shown in the shooting phase. */
export interface LastShotSummary {
	frame: string;
	aperture: string | null;
	shutter: string | null;
	lensName: string | null;
}

/**
 * Summarize the most recent shot (highest frame, matching the roll page's
 * date-default convention) for the shooting-phase reference card. Null when the
 * roll has no shots. `lensName` is null when the shot carries no lens or its lens
 * id is not in the catalog.
 */
export function lastShotSummary(
	shots: Shot[],
	shotLensMap: Record<number, number[]>,
	lenses: Lens[]
): LastShotSummary | null {
	if (shots.length === 0) return null;
	const shot = shots[shots.length - 1];
	const lensIds = shotLensMap[shot.id] ?? [];
	const lens = lensIds.length > 0 ? (lenses.find((l) => l.id === lensIds[0]) ?? null) : null;
	return {
		frame: shot.frame_number,
		aperture: shot.aperture,
		shutter: shot.shutter_speed,
		lensName: lens ? lensDisplayName(lens) : null
	};
}
