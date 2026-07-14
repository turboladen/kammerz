import type { Chemical } from '$lib/types';

/** The `default_dilution` of the named chemical in `list`, or null if unknown. */
export function defaultDilutionFor(list: Chemical[], name: string): string | null {
	return list.find((c) => c.name === name)?.default_dilution ?? null;
}

/**
 * Decide what a dilution field should become when a known chemical is selected.
 * Returns the value to apply, or null to leave the field unchanged — a non-empty
 * current dilution is never overwritten, and a chemical with no default is a
 * no-op (kammerz-9fx).
 */
export function dilutionPrefill(current: string, chemicalDefault: string | null | undefined): string | null {
	if (current.trim() !== '') return null;
	if (!chemicalDefault || chemicalDefault.trim() === '') return null;
	return chemicalDefault;
}
