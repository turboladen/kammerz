/** Convert seconds integer to "m:ss" display string. Returns '' for null/undefined. */
export function secondsToMmSs(seconds: number | null): string {
	if (seconds == null) return '';
	const m = Math.floor(seconds / 60);
	const s = seconds % 60;
	return `${m}:${s.toString().padStart(2, '0')}`;
}

/** Parse "m:ss" or "m" string to seconds. Returns null for empty/invalid input. */
export function mmSsToSeconds(str: string): number | null {
	if (!str.trim()) return null;
	const parts = str.split(':');
	if (parts.length === 2) {
		const m = parseInt(parts[0], 10);
		const s = parseInt(parts[1], 10);
		if (isNaN(m) || isNaN(s)) return null;
		return m * 60 + s;
	}
	if (parts.length === 1) {
		const m = parseInt(parts[0], 10);
		if (isNaN(m)) return null;
		return m * 60;
	}
	return null;
}
