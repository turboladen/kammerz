import type { RollWithDetails, DevelopmentLab, DevelopmentSelf, RollStatus } from '$lib/types';

/** Where a milestone's date is stored, so the editor/prompt can write it back. */
export type DateTarget =
	| {
			kind: 'roll';
			field: 'date_loaded' | 'date_finished' | 'date_scanned' | 'date_post_processed' | 'date_archived';
	  }
	| { kind: 'lab'; field: 'date_dropped_off' | 'date_received' }
	| { kind: 'self'; field: 'date_processed' };

/** Read a target's current date from whichever record owns it. A lab/self target with no
 *  dev record reads as null (the column lives on that record). */
export function readDateTarget(
	t: DateTarget,
	roll: RollWithDetails,
	labDev: DevelopmentLab | null,
	selfDev: DevelopmentSelf | null
): string | null {
	if (t.kind === 'roll') return roll[t.field] ?? null;
	if (t.kind === 'lab') return labDev?.[t.field] ?? null;
	return selfDev?.[t.field] ?? null;
}

/**
 * Status → date target. Used by the roll detail page to decide which date a forward
 * status transition records, and where it lives.
 */
export const STATUS_DATE_TARGET: Partial<Record<RollStatus, DateTarget>> = {
	shot: { kind: 'roll', field: 'date_finished' },
	'lab-done': { kind: 'lab', field: 'date_received' },
	developed: { kind: 'self', field: 'date_processed' },
	scanned: { kind: 'roll', field: 'date_scanned' },
	'post-processed': { kind: 'roll', field: 'date_post_processed' },
	archived: { kind: 'roll', field: 'date_archived' }
};
