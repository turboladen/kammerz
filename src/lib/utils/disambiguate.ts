import type { Camera } from '$lib/types';

interface Identifiable {
	id: number;
	serial_number?: string | null;
	created_at: string;
}

/**
 * When multiple items share the same display label, appends a suffix to distinguish them.
 * Items with serial numbers get "(S/N xxxxx)"; others get "(Copy N)" by creation order.
 * Single-instance labels are left unchanged.
 */
export function buildDisambiguatedLabels<T extends Identifiable>(
	items: T[],
	getLabel: (item: T) => string
): Map<number, string> {
	const result = new Map<number, string>();

	// Group items by their base label
	const groups = new Map<string, T[]>();
	for (const item of items) {
		const label = getLabel(item);
		const group = groups.get(label) ?? [];
		group.push(item);
		groups.set(label, group);
	}

	for (const [label, group] of groups) {
		if (group.length === 1) {
			result.set(group[0].id, label);
		} else {
			// Sort by creation date for deterministic Copy numbering
			const sorted = [...group].sort((a, b) => a.created_at.localeCompare(b.created_at));
			let copyCounter = 1;
			for (const item of sorted) {
				if (item.serial_number) {
					result.set(item.id, `${label} (S/N ${item.serial_number})`);
				} else {
					result.set(item.id, `${label} (Copy ${copyCounter})`);
					copyCounter++;
				}
			}
		}
	}

	return result;
}

export function buildCameraLabels(cameras: Camera[]): Map<number, string> {
	return buildDisambiguatedLabels(cameras, (c) => `${c.brand} ${c.model}`);
}
