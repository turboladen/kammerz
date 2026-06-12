/**
 * Pure list-processing utilities for search, sort, and grouping.
 * Used in $derived chains on all list pages.
 */

/** Filter items by full-text search. Returns all items when query is empty. */
export function filterBySearch<T>(items: T[], query: string, getSearchText: (item: T) => string): T[] {
	const q = query.trim().toLowerCase();
	if (!q) return items;
	return items.filter((item) => getSearchText(item).toLowerCase().includes(q));
}

/**
 * Group a pre-sorted array into an ordered Record<string, T[]>.
 * Key order reflects first-appearance in items (i.e. the sort order).
 */
export function groupItems<T>(items: T[], getKey: (item: T) => string): Record<string, T[]> {
	const result: Record<string, T[]> = {};
	for (const item of items) {
		const key = getKey(item);
		if (!result[key]) result[key] = [];
		result[key].push(item);
	}
	return result;
}

/** Sort by a string key. Null/undefined values sort last (empty strings sort normally). */
export function sortByString<T>(
	items: T[],
	getKey: (item: T) => string | null | undefined,
	direction: 'asc' | 'desc' = 'asc'
): T[] {
	return [...items].sort((a, b) => {
		const ka = getKey(a)?.toLowerCase();
		const kb = getKey(b)?.toLowerCase();
		if (ka == null && kb == null) return 0;
		if (ka == null) return 1;
		if (kb == null) return -1;
		const cmp = ka < kb ? -1 : ka > kb ? 1 : 0;
		return direction === 'asc' ? cmp : -cmp;
	});
}

/** Sort by a date string (YYYY-MM-DD or ISO). Null/empty values sort last. */
export function sortByDate<T>(
	items: T[],
	getKey: (item: T) => string | null | undefined,
	direction: 'asc' | 'desc' = 'desc'
): T[] {
	return [...items].sort((a, b) => {
		const ka = getKey(a) ?? '';
		const kb = getKey(b) ?? '';
		if (!ka && !kb) return 0;
		if (!ka) return 1;
		if (!kb) return -1;
		const cmp = ka < kb ? -1 : ka > kb ? 1 : 0;
		return direction === 'asc' ? cmp : -cmp;
	});
}

/** Sort by a numeric key. Null values sort last. */
export function sortByNumber<T>(
	items: T[],
	getKey: (item: T) => number | null | undefined,
	direction: 'asc' | 'desc' = 'asc'
): T[] {
	return [...items].sort((a, b) => {
		const ka = getKey(a);
		const kb = getKey(b);
		if (ka == null && kb == null) return 0;
		if (ka == null) return 1;
		if (kb == null) return -1;
		return direction === 'asc' ? ka - kb : kb - ka;
	});
}
