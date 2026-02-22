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

/** Ordered list of all statuses (matches the roll lifecycle progression). */
export const statusOrder: RollStatus[] = [
	'loaded',
	'shooting',
	'shot',
	'at-lab',
	'developing',
	'developed',
	'scanned',
	'archived'
];

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
