import type { RollWithDetails, DevelopmentLab, DevelopmentSelf, RollStatus } from '$lib/types';
import { allStatusOrder, getFlowForPath, statusConfig, type DevPath } from './status';

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
	key: MilestoneKey;
	label: string;
	date: string | null;
	/** Which record + column this date lives in (edit/transition write target). */
	target: DateTarget;
	/** Whether this date can be set/edited inline. False when the backing record doesn't
	 *  exist yet (lab/self before a dev record) OR the roll hasn't reached this rung yet —
	 *  future dates are recorded by advancing the status, not back-filled (kammerz-fxl). */
	editable: boolean;
}

/** The fixed set of lifecycle milestones. Not every roll hits all of them — the
 *  lab/self middle is mutually exclusive (see MILESTONE_ORDER). */
export type MilestoneKey =
	| 'loaded'
	| 'finished-shooting'
	| 'dropped-off'
	| 'received'
	| 'developed'
	| 'scanned'
	| 'post-processed'
	| 'archived';

/**
 * The single source of truth for each milestone's label and where its date lives.
 * Dates come from heterogeneous records: the roll row owns loaded / finished-shooting /
 * scanned / post-processed / archived, while the development middle is owned by the dev
 * records (lab: dropped-off + received; self: developed = date_processed). Both
 * `buildRollTimeline` and the status→date mapping below derive from this — add a milestone
 * here once, not in two places.
 */
const MILESTONE_DEFS: Record<MilestoneKey, { label: string; target: DateTarget }> = {
	loaded: { label: 'Loaded', target: { kind: 'roll', field: 'date_loaded' } },
	'finished-shooting': { label: 'Finished shooting', target: { kind: 'roll', field: 'date_finished' } },
	'dropped-off': { label: 'Dropped off at lab', target: { kind: 'lab', field: 'date_dropped_off' } },
	received: { label: 'Received back', target: { kind: 'lab', field: 'date_received' } },
	developed: { label: 'Developed', target: { kind: 'self', field: 'date_processed' } },
	scanned: { label: 'Scanned', target: { kind: 'roll', field: 'date_scanned' } },
	'post-processed': { label: 'Post-processed', target: { kind: 'roll', field: 'date_post_processed' } },
	archived: { label: 'Archived', target: { kind: 'roll', field: 'date_archived' } }
};

/** Ordered, path-aware milestone keys. Mirrors labFlow/selfFlow in status.ts; the lab/self
 *  middle is mutually exclusive and the undecided path omits it. */
const MILESTONE_ORDER: Record<DevPath, MilestoneKey[]> = {
	lab: ['loaded', 'finished-shooting', 'dropped-off', 'received', 'scanned', 'post-processed', 'archived'],
	self: ['loaded', 'finished-shooting', 'developed', 'scanned', 'post-processed', 'archived'],
	undecided: ['loaded', 'finished-shooting', 'scanned', 'post-processed', 'archived']
};

/**
 * The status whose arrival marks each milestone as reached. A milestone's date is only
 * editable once the roll has advanced to (or past) this status — recording a future rung's
 * date is done by advancing the status (which writes the date AND runs backend auto-sync),
 * never by back-filling a date on a state the roll isn't in (kammerz-fxl). The lab/self
 * middle uses the entry status of its dev work (at-lab / developed) so the date can be
 * corrected as soon as that stage is current.
 */
const MILESTONE_STATUS: Record<MilestoneKey, RollStatus> = {
	loaded: 'loaded',
	'finished-shooting': 'shot',
	'dropped-off': 'at-lab',
	received: 'lab-done',
	developed: 'developed',
	scanned: 'scanned',
	'post-processed': 'post-processed',
	archived: 'archived'
};

/** Whether the roll has reached a milestone's rung, using the canonical union ordering so the
 *  comparison is robust regardless of dev path. An unknown current status (not in the order)
 *  counts as before everything, so only already-dated milestones stay editable. */
function milestoneReached(key: MilestoneKey, status: RollStatus): boolean {
	const current = allStatusOrder.indexOf(status);
	return current >= 0 && current >= allStatusOrder.indexOf(MILESTONE_STATUS[key]);
}

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

/** Whether a target's backing record exists, so its date can be written/edited. Roll dates
 *  are always editable; lab/self dates need their dev record to exist first. */
export function dateTargetEditable(
	t: DateTarget,
	labDev: DevelopmentLab | null,
	selfDev: DevelopmentSelf | null
): boolean {
	if (t.kind === 'roll') return true;
	if (t.kind === 'lab') return labDev != null;
	return selfDev != null;
}

/**
 * Build the ordered, path-aware lifecycle timeline for a roll. Each milestone's label and
 * target come from MILESTONE_DEFS; its date comes from the target and its editability from
 * both the target's backing record AND whether the roll has reached that rung. A null date
 * renders as an undated milestone — either not yet reached, or reached but cleared from the
 * Timeline. Not-yet-reached rungs are non-editable: their date is set by advancing the
 * status, not by back-filling here (kammerz-fxl).
 */
export function buildRollTimeline(
	roll: RollWithDetails,
	labDev: DevelopmentLab | null,
	selfDev: DevelopmentSelf | null,
	devPath: DevPath
): TimelineMilestone[] {
	return MILESTONE_ORDER[devPath].map((key) => {
		const { label, target } = MILESTONE_DEFS[key];
		return {
			key,
			label,
			target,
			date: readDateTarget(target, roll, labDev, selfDev),
			editable: dateTargetEditable(target, labDev, selfDev) && milestoneReached(key, roll.status)
		};
	});
}

/** Progression state of a lifecycle rung relative to the roll's current status. */
export type RungState = 'past' | 'current' | 'future';

/**
 * One rung of the unified lifecycle stepper: a status (the transition target) joined to its
 * dated milestone, if it has one. Statuses that carry no date of their own — `shooting`,
 * `developing` — have `milestone: null` and render dateless. The status is the key (so the
 * rung stays clickable to transition); the milestone supplies the date + inline editor.
 */
export interface LifecycleRung {
	status: RollStatus;
	/** Status label (statusConfig) — the rung is a transition target, so it reads as the status. */
	label: string;
	state: RungState;
	milestone: TimelineMilestone | null;
}

/**
 * Build the unified, path-aware lifecycle: every status in the roll's flow, each joined to its
 * dated milestone for display + inline editing. Reuses `buildRollTimeline` for the milestone
 * data (dates + the kammerz-fxl editability gating) and indexes it by status via
 * `MILESTONE_STATUS`, so the date shown on a rung is the event that lands the roll there
 * (e.g. `at-lab` shows "Dropped off at lab", `lab-done` shows "Received back"). Merges what
 * used to be two separate sections (the STATUS chevron bar and the TIMELINE list) — kammerz-06i.
 */
export function buildRollLifecycle(
	roll: RollWithDetails,
	labDev: DevelopmentLab | null,
	selfDev: DevelopmentSelf | null,
	devPath: DevPath
): LifecycleRung[] {
	const byStatus = new Map<RollStatus, TimelineMilestone>();
	for (const m of buildRollTimeline(roll, labDev, selfDev, devPath)) {
		byStatus.set(MILESTONE_STATUS[m.key], m);
	}
	const flow = getFlowForPath(devPath);
	const currentIdx = flow.indexOf(roll.status);
	return flow.map((status, idx) => ({
		status,
		label: statusConfig[status].label,
		state: idx === currentIdx ? 'current' : idx < currentIdx ? 'past' : 'future',
		milestone: byStatus.get(status) ?? null
	}));
}

/**
 * Which milestone a forward transition into each status records. A total Record (not Partial)
 * so a newly added RollStatus is a compile error until it's classified here — preventing the
 * silent "no date prompt" gap. `null` means no transition prompt: loaded/shooting are
 * implicit, and at-lab/developing capture their dates through the full dev dialogs instead.
 */
const STATUS_MILESTONE: Record<RollStatus, MilestoneKey | null> = {
	loaded: null,
	shooting: null,
	shot: 'finished-shooting',
	'at-lab': null,
	'lab-done': 'received',
	developing: null,
	developed: 'developed',
	scanned: 'scanned',
	'post-processed': 'post-processed',
	archived: 'archived'
};

/**
 * Status → date target, derived from STATUS_MILESTONE + MILESTONE_DEFS. Used by the roll
 * detail page to decide which date a forward status transition records, and where it lives.
 */
export const STATUS_DATE_TARGET: Partial<Record<RollStatus, DateTarget>> = (() => {
	const map: Partial<Record<RollStatus, DateTarget>> = {};
	// STATUS_MILESTONE is a total Record<RollStatus, …>, so its keys are exactly RollStatus.
	for (const status of Object.keys(STATUS_MILESTONE) as RollStatus[]) {
		const key = STATUS_MILESTONE[status];
		// `if (key !== null)` narrows `key` to MilestoneKey here (control-flow narrowing on a
		// plain variable, unlike a destructured tuple in .filter) — so no `key as MilestoneKey`.
		if (key !== null) map[status] = MILESTONE_DEFS[key].target;
	}
	return map;
})();
