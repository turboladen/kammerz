import type { RollStatus } from '$lib/types';

/**
 * Canonical status metadata — single source of truth for all status display info.
 *
 * Every component that needs status labels, colors, or CSS classes should import
 * from here rather than defining its own inline map. This prevents key/label
 * mismatches (e.g., using display labels where stored values are expected).
 *
 * Keys are RollStatus values (lowercase hyphenated, matching the database).
 */
export const statusConfig: Record<
	RollStatus,
	{
		label: string;
		colorVar: string;
		pillClasses: string;
		dotClass: string;
	}
> = {
	loaded: {
		label: 'Loaded',
		colorVar: 'var(--color-status-loaded)',
		pillClasses: 'bg-status-loaded/10 text-status-loaded',
		dotClass: 'bg-status-loaded'
	},
	shooting: {
		label: 'Shooting',
		colorVar: 'var(--color-status-shooting)',
		pillClasses: 'bg-status-shooting/10 text-status-shooting',
		dotClass: 'bg-status-shooting'
	},
	shot: {
		label: 'Shot',
		colorVar: 'var(--color-status-shot)',
		pillClasses: 'bg-status-shot/10 text-status-shot',
		dotClass: 'bg-status-shot'
	},
	'at-lab': {
		label: 'At Lab',
		colorVar: 'var(--color-status-at-lab)',
		pillClasses: 'bg-status-at-lab/10 text-status-at-lab',
		dotClass: 'bg-status-at-lab'
	},
	'lab-done': {
		label: 'Lab Done',
		colorVar: 'var(--color-status-lab-done)',
		pillClasses: 'bg-status-lab-done/10 text-status-lab-done',
		dotClass: 'bg-status-lab-done'
	},
	developing: {
		label: 'Developing',
		colorVar: 'var(--color-status-developing)',
		pillClasses: 'bg-status-developing/10 text-status-developing',
		dotClass: 'bg-status-developing'
	},
	developed: {
		label: 'Developed',
		colorVar: 'var(--color-status-developed)',
		pillClasses: 'bg-status-developed/10 text-status-developed',
		dotClass: 'bg-status-developed'
	},
	scanned: {
		label: 'Scanned',
		colorVar: 'var(--color-status-scanned)',
		pillClasses: 'bg-status-scanned/10 text-status-scanned',
		dotClass: 'bg-status-scanned'
	},
	archived: {
		label: 'Archived',
		colorVar: 'var(--color-status-archived)',
		pillClasses: 'bg-status-archived/10 text-status-archived',
		dotClass: 'bg-status-archived'
	}
};

// ---------------------------------------------------------------------------
// Path-specific status flows
// ---------------------------------------------------------------------------

/** Lab development path: Shot → At Lab → Lab Done → Scanned → Archived */
export const labFlow: RollStatus[] = [
	'loaded',
	'shooting',
	'shot',
	'at-lab',
	'lab-done',
	'scanned',
	'archived'
];

/** Self development path: Shot → Developing → Developed → Scanned → Archived */
export const selfFlow: RollStatus[] = [
	'loaded',
	'shooting',
	'shot',
	'developing',
	'developed',
	'scanned',
	'archived'
];

/** Undecided path (no dev record): shows shared prefix + suffix with a visual gap. */
export const undecidedFlow: RollStatus[] = ['loaded', 'shooting', 'shot', 'scanned', 'archived'];

/**
 * Combined sort order — includes ALL statuses for cross-roll sorting contexts
 * (dashboard "In the Darkroom", rolls list group-by-status, status distribution bar).
 * Lab-path statuses are interleaved before self-path at their natural position.
 */
export const allStatusOrder: RollStatus[] = [
	'loaded',
	'shooting',
	'shot',
	'at-lab',
	'lab-done',
	'developing',
	'developed',
	'scanned',
	'archived'
];

/** @deprecated Use allStatusOrder or getFlowForPath() for path-specific rendering. */
export const statusOrder = allStatusOrder;

// ---------------------------------------------------------------------------
// Path determination
// ---------------------------------------------------------------------------

/** Which development workflow a roll is following. */
export type DevPath = 'lab' | 'self' | 'undecided';

/**
 * Determine which development path a roll is on.
 *
 * Priority:
 * 1. Dev record exists → that path
 * 2. Current status belongs to a specific path → that path (orphaned edge case)
 * 3. Otherwise → undecided
 */
export function getDevPath(
	status: RollStatus,
	hasLabDev: boolean,
	hasSelfDev: boolean
): DevPath {
	if (hasLabDev) return 'lab';
	if (hasSelfDev) return 'self';
	if (status === 'at-lab' || status === 'lab-done') return 'lab';
	if (status === 'developing' || status === 'developed') return 'self';
	return 'undecided';
}

/** Get the status flow array for a given development path. */
export function getFlowForPath(path: DevPath): RollStatus[] {
	switch (path) {
		case 'lab':
			return labFlow;
		case 'self':
			return selfFlow;
		case 'undecided':
			return undecidedFlow;
	}
}

/** Get a human-readable label for the development path (null when undecided). */
export function getPathLabel(path: DevPath): string | null {
	switch (path) {
		case 'lab':
			return 'Lab Development';
		case 'self':
			return 'Self Development';
		case 'undecided':
			return null;
	}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Type guard: check if a string is a valid RollStatus. */
export function isRollStatus(value: string): value is RollStatus {
	return value in statusConfig;
}

/** Get the CSS custom property color for a status. */
export function getStatusColor(status: RollStatus): string {
	return statusConfig[status].colorVar;
}

/**
 * Safe color lookup for untyped status strings (e.g., from backend RankedItem.label).
 * Returns accent color as fallback for unknown values.
 */
export function getStatusColorSafe(status: string): string {
	return isRollStatus(status) ? statusConfig[status].colorVar : 'var(--color-accent)';
}

/** Get the human-readable label for a status. */
export function getStatusLabel(status: RollStatus): string {
	return statusConfig[status].label;
}
