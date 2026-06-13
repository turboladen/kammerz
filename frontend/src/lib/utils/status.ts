import type { RollStatus } from '$lib/types';
import statusFlows from '$lib/status-flows.json';

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
	'post-processed': {
		label: 'Post-processed',
		colorVar: 'var(--color-status-post-processed)',
		pillClasses: 'bg-status-post-processed/10 text-status-post-processed',
		dotClass: 'bg-status-post-processed'
	},
	archived: {
		label: 'Archived',
		colorVar: 'var(--color-status-archived)',
		pillClasses: 'bg-status-archived/10 text-status-archived',
		dotClass: 'bg-status-archived'
	}
};

// ---------------------------------------------------------------------------
// Path-specific status flows — DERIVED from the canonical fixture
// ---------------------------------------------------------------------------
//
// The flow arrays below are not hand-written here: they are read from
// `$lib/status-flows.json`, the single committed source of the status ordering.
// The same fixture is asserted against the Rust `LAB_FLOW`/`SELF_FLOW` constants
// and the `RollStatus` enum's full variant ordering by `tests/status_flows.rs`,
// so any drift between the frontend arrays, the backend flows, and the enum
// fails `cargo test`. See CLAUDE.md (status-flow / enum-sync conventions).
//
// `status-flows.json` is typed by the co-located `status-flows.d.ts` so these
// arrays import as `RollStatus[]` rather than the widened `string[]` a raw JSON
// import yields. The fixture's CONTENTS (every entry being a real status, the
// flows matching the backend) are gated by `tests/status_flows.rs`, which asserts
// the fixture against the `RollStatus` enum and the `LAB_FLOW`/`SELF_FLOW`
// constants — a typo or reordering there fails `cargo test`. `assertStatusFixture`
// below is the frontend's complementary runtime guard (it runs in the browser /
// dev and backs any future frontend unit test), binding the fixture to the
// `RollStatus` union via `statusConfig`'s keys.

/** Lab development path: Shot → At Lab → Lab Done → Scanned → Archived */
export const labFlow = statusFlows.labFlow;

/** Self development path: Shot → Developing → Developed → Scanned → Archived */
export const selfFlow = statusFlows.selfFlow;

/** Undecided path (no dev record): shows shared prefix + suffix with a visual gap. */
export const undecidedFlow = statusFlows.undecidedFlow;

/**
 * Combined sort order — includes ALL statuses for cross-roll sorting contexts
 * (dashboard "In the Darkroom", rolls list group-by-status, status distribution bar).
 * Lab-path statuses are interleaved before self-path at their natural position.
 */
export const allStatusOrder = statusFlows.statuses;

/**
 * Runtime cross-check that the fixture's `statuses` and the `RollStatus` union
 * (as enumerated by `statusConfig`'s keys) describe exactly the same set, and
 * that each flow is an order-preserving subsequence of `statuses`. Throws at
 * module load on drift — every fixture status must be a known config key, every
 * config key must appear in the fixture, and each flow must list a subset of the
 * canonical statuses in canonical order. This is the frontend half of the
 * cross-check; the Rust `tests/status_flows.rs` half binds the fixture to the
 * backend `LAB_FLOW`/`SELF_FLOW` constants and the enum ordering — but the Rust
 * side does not cover `undecidedFlow` (frontend-only, no backend constant), so
 * this subsequence check is the only guard on `undecidedFlow`'s ordering.
 */
function assertStatusFixture(): void {
	const configKeys = new Set(Object.keys(statusConfig));
	const canonicalIndex = new Map<string, number>(statusFlows.statuses.map((s, i) => [s, i]));

	for (const status of statusFlows.statuses) {
		if (!configKeys.has(status)) {
			throw new Error(`status-flows.json lists unknown status "${status}" not in statusConfig (RollStatus union)`);
		}
	}
	for (const key of configKeys) {
		if (!canonicalIndex.has(key)) {
			throw new Error(`RollStatus "${key}" is missing from status-flows.json "statuses"`);
		}
	}
	for (const [name, flow] of [
		['labFlow', statusFlows.labFlow],
		['selfFlow', statusFlows.selfFlow],
		['undecidedFlow', statusFlows.undecidedFlow]
	] as const) {
		let prev = -1;
		for (const status of flow) {
			const idx = canonicalIndex.get(status);
			if (idx === undefined) {
				throw new Error(`status-flows.json ${name} references "${status}" not in "statuses"`);
			}
			if (idx <= prev) {
				throw new Error(`status-flows.json ${name} is not in canonical "statuses" order at "${status}"`);
			}
			prev = idx;
		}
	}
}

assertStatusFixture();

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
export function getDevPath(status: RollStatus, hasLabDev: boolean, hasSelfDev: boolean): DevPath {
	if (hasLabDev) return 'lab';
	if (hasSelfDev) return 'self';
	// No dev record yet — fall back to the status's intrinsic dev kind (orphaned edge case).
	return devKindForStatus(status) ?? 'undecided';
}

/**
 * Request to auto-open a dev dialog (chevron click / Develop menu). `target` is the
 * status chevron the user actually clicked (when any) so the dialog can seed the
 * date field that lands the roll there and explain where saving will move it —
 * without it, a "Lab Done" click silently lands at At Lab (kammerz-zoo).
 */
export type DevAutoPrompt = { kind: 'lab' | 'self'; target?: RollStatus };

/**
 * Which development record a status belongs to, or null for roll-owned / undecided statuses.
 * The single source of the status→record-kind inference (also used by {@link getDevPath}) — a
 * pure lookup independent of whether the record exists. Used to gate "open the dev dialog before
 * advancing" so a forward transition never strands a status with no backing dev record.
 */
export function devKindForStatus(status: RollStatus): 'lab' | 'self' | null {
	if (status === 'at-lab' || status === 'lab-done') return 'lab';
	if (status === 'developing' || status === 'developed') return 'self';
	return null;
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
