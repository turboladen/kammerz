import type { RollWithDetails, DevelopmentLab, DevelopmentSelf } from '$lib/types';
import type { DevPath } from './status';

/** Where a milestone's date is stored, so the editor/prompt can write it back. */
export type DateTarget =
	| {
			kind: 'roll';
			field: 'date_loaded' | 'date_finished' | 'date_scanned' | 'date_post_processed' | 'date_archived';
	  }
	| { kind: 'lab'; field: 'date_dropped_off' | 'date_received' }
	| { kind: 'self'; field: 'date_processed' };

/** One dated milestone in a roll's lifecycle. `date` is null when the milestone hasn't been
 *  reached yet — or when it was reached but its date was intentionally cleared in the Timeline. */
export interface TimelineMilestone {
	key: string;
	label: string;
	date: string | null;
	/** Which record + column this date lives in (edit/transition write target). */
	target: DateTarget;
	/** False when the backing record doesn't exist yet (lab/self before a dev record). */
	editable: boolean;
}

/**
 * Build the ordered, path-aware lifecycle timeline for a roll.
 *
 * Dates come from heterogeneous sources: the roll row owns loaded / finished-shooting /
 * scanned / post-processed / archived, while the development middle is owned by the dev records
 * (lab: dropped-off + received-back; self: developed = date_processed). The lab/self middle is
 * mutually exclusive and mirrors labFlow/selfFlow in status.ts; the undecided path omits the
 * middle. A null date renders as an undated milestone — either not yet reached, or reached
 * but cleared from the Timeline.
 */
export function buildRollTimeline(
	roll: RollWithDetails,
	labDev: DevelopmentLab | null,
	selfDev: DevelopmentSelf | null,
	devPath: DevPath
): TimelineMilestone[] {
	const milestones: TimelineMilestone[] = [
		{ key: 'loaded', label: 'Loaded', date: roll.date_loaded, target: { kind: 'roll', field: 'date_loaded' }, editable: true },
		{ key: 'finished-shooting', label: 'Finished shooting', date: roll.date_finished, target: { kind: 'roll', field: 'date_finished' }, editable: true }
	];

	if (devPath === 'lab') {
		milestones.push(
			{ key: 'dropped-off', label: 'Dropped off at lab', date: labDev?.date_dropped_off ?? null, target: { kind: 'lab', field: 'date_dropped_off' }, editable: labDev != null },
			{ key: 'received', label: 'Received back', date: labDev?.date_received ?? null, target: { kind: 'lab', field: 'date_received' }, editable: labDev != null }
		);
	} else if (devPath === 'self') {
		milestones.push({ key: 'developed', label: 'Developed', date: selfDev?.date_processed ?? null, target: { kind: 'self', field: 'date_processed' }, editable: selfDev != null });
	}

	milestones.push(
		{ key: 'scanned', label: 'Scanned', date: roll.date_scanned, target: { kind: 'roll', field: 'date_scanned' }, editable: true },
		{ key: 'post-processed', label: 'Post-processed', date: roll.date_post_processed, target: { kind: 'roll', field: 'date_post_processed' }, editable: true },
		{ key: 'archived', label: 'Archived', date: roll.date_archived, target: { kind: 'roll', field: 'date_archived' }, editable: true }
	);

	return milestones;
}
