import type { RollWithDetails, DevelopmentLab, DevelopmentSelf } from '$lib/types';
import type { DevPath } from './status';

/** One dated milestone in a roll's lifecycle. `date` is null until the milestone is reached. */
export interface TimelineMilestone {
	key: string;
	label: string;
	date: string | null;
}

/**
 * Build the ordered, path-aware lifecycle timeline for a roll.
 *
 * The dates come from heterogeneous sources: the roll row owns loaded / finished-shooting /
 * scanned / post-processed / archived, while the development middle is owned by the dev records
 * (lab: dropped-off + received-back; self: developed = date_processed). The lab/self middle is
 * mutually exclusive and mirrors labFlow/selfFlow in status.ts; the undecided path simply omits
 * the middle. A null date renders as a not-yet-reached milestone.
 */
export function buildRollTimeline(
	roll: RollWithDetails,
	labDev: DevelopmentLab | null,
	selfDev: DevelopmentSelf | null,
	devPath: DevPath
): TimelineMilestone[] {
	const milestones: TimelineMilestone[] = [
		{ key: 'loaded', label: 'Loaded', date: roll.date_loaded },
		{ key: 'finished-shooting', label: 'Finished shooting', date: roll.date_finished }
	];

	if (devPath === 'lab') {
		milestones.push(
			{ key: 'dropped-off', label: 'Dropped off at lab', date: labDev?.date_dropped_off ?? null },
			{ key: 'received', label: 'Received back', date: labDev?.date_received ?? null }
		);
	} else if (devPath === 'self') {
		milestones.push({ key: 'developed', label: 'Developed', date: selfDev?.date_processed ?? null });
	}

	milestones.push(
		{ key: 'scanned', label: 'Scanned', date: roll.date_scanned },
		{ key: 'post-processed', label: 'Post-processed', date: roll.date_post_processed },
		{ key: 'archived', label: 'Archived', date: roll.date_archived }
	);

	return milestones;
}
