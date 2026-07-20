// Lifecycle-phase display metadata (ADR-0013), replacing the retired per-status
// `statusConfig`. The backend derives a `group_key` (0..=4 = the earliest
// unresolved activity, 5 = Done) and a compound `badge` string for every roll;
// the frontend never re-derives — it only colors and labels by phase here.
//
// A roll's Badge is colored by its `group_key` phase, NOT by the compound badge
// text (which can list several in-progress activities, e.g. "Scanning ·
// Post-processing"). Ten legacy statuses collapse to six phases: at-lab and
// developing both render as Development, lab-done and developed both as "To
// scan"'s Scanning phase — the lab/self distinction lives on the dev record, not
// the lifecycle position.

/** One lifecycle phase's display metadata, indexed by `group_key`. */
export interface PhaseMeta {
	/** The `group_key` this entry describes (0..=5). */
	groupKey: number;
	/** Human phase name. MUST match the backend `PHASE_LABELS` (activity.rs) so the
	 *  stats "Rolls by Phase" panel can color each bar via {@link phaseByLabel}. */
	label: string;
	/** CSS custom-property color for bars/dots (e.g. the stats + dashboard charts). */
	colorVar: string;
	/** Literal Tailwind classes for the Badge pill (bg at 10% + text). Written out
	 *  in full so the Tailwind JIT sees every class — never build these by
	 *  interpolation. */
	pillClasses: string;
	/** Literal Tailwind class for the small color dot before the label. */
	dotClass: string;
}

/**
 * The six lifecycle phases in `group_key` order. Reuses the existing
 * `--color-status-*` tokens (grouped on the red/green-safe blue↔yellow axis):
 * shooting cool azure → development amber → scanning cool slate → post-processing
 * violet → archiving/done neutral taupe. Done reuses the archived taupe (a
 * distinct green would break the red/green-safe rule).
 */
export const PHASE_META: readonly PhaseMeta[] = [
	{
		groupKey: 0,
		label: 'Shooting',
		colorVar: 'var(--color-status-loaded)',
		pillClasses: 'bg-status-loaded/10 text-status-loaded',
		dotClass: 'bg-status-loaded'
	},
	{
		groupKey: 1,
		label: 'Development',
		colorVar: 'var(--color-status-at-lab)',
		pillClasses: 'bg-status-at-lab/10 text-status-at-lab',
		dotClass: 'bg-status-at-lab'
	},
	{
		groupKey: 2,
		label: 'Scanning',
		colorVar: 'var(--color-status-scanned)',
		pillClasses: 'bg-status-scanned/10 text-status-scanned',
		dotClass: 'bg-status-scanned'
	},
	{
		groupKey: 3,
		label: 'Post-processing',
		colorVar: 'var(--color-status-post-processed)',
		pillClasses: 'bg-status-post-processed/10 text-status-post-processed',
		dotClass: 'bg-status-post-processed'
	},
	{
		groupKey: 4,
		label: 'Archiving',
		colorVar: 'var(--color-status-archived)',
		pillClasses: 'bg-status-archived/10 text-status-archived',
		dotClass: 'bg-status-archived'
	},
	{
		groupKey: 5,
		label: 'Done',
		colorVar: 'var(--color-status-archived)',
		pillClasses: 'bg-status-archived/10 text-status-archived',
		dotClass: 'bg-status-archived'
	}
] as const;

/** The phase metadata for a `group_key`. Out-of-range keys clamp to Done (the
 *  group_key domain is 0..=5, so this is a defensive fallback). */
export function phaseTheme(groupKey: number): PhaseMeta {
	return PHASE_META[groupKey] ?? PHASE_META[PHASE_META.length - 1];
}

/** The human phase name for a `group_key`. */
export function phaseLabel(groupKey: number): string {
	return phaseTheme(groupKey).label;
}

/** Look up a phase by its label — for the stats "Rolls by Phase" panel, whose
 *  rows arrive from the backend as `{ label, count }`. Returns undefined for an
 *  unrecognized label so callers can fall back to a neutral color. */
export function phaseByLabel(label: string): PhaseMeta | undefined {
	return PHASE_META.find((p) => p.label === label);
}
