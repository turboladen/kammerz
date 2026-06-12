import type { Lens, LensMount } from '$lib/types';
import { buildDisambiguatedLabels } from './disambiguate';

/** Minimal shape needed to render a lens display name — satisfied by both Lens and LensSearchResult. */
export type LensNameParts = Pick<Lens, 'brand' | 'model'> & Partial<Pick<Lens, 'focal_length' | 'max_aperture'>>;

export function lensDisplayName(lens: LensNameParts): string {
	if (lens.model) {
		// Avoid doubling brand when model starts with it (e.g. "Mamiya 90mm K/L")
		if (lens.model.toLowerCase().startsWith(lens.brand.toLowerCase())) return lens.model;
		return `${lens.brand} ${lens.model}`;
	}
	const parts = [lens.brand];
	if (lens.focal_length) parts.push(`${lens.focal_length}mm`);
	if (lens.max_aperture) parts.push(`f/${lens.max_aperture}`);
	return parts.join(' ');
}

/** Returns true if the mount name belongs to the large format family. */
export function isLargeFormatMount(name: string): boolean {
	return (
		name.startsWith('Copal') || name.startsWith('Compur') || name.startsWith('Barrel') || name.includes('Large Format')
	);
}

/** Build a Set of mount IDs that belong to the large format family. */
function buildLfFamilyIds(mounts: LensMount[]): Set<number> {
	return new Set(mounts.filter((m) => isLargeFormatMount(m.name)).map((m) => m.id));
}

/** Returns true if the mount name belongs to the medium format family. */
function isMediumFormatMount(name: string): boolean {
	return name.includes('Pentax 67') || name.includes('Hasselblad') || name.includes('Mamiya');
}

type SelectOption = { value: string; label: string; disabled?: boolean };

/** Sentinel mount-Select value that reveals the inline "create a new mount" form. */
export const NEW_MOUNT_OPTION = '__new__';

/** Returns true if the mount is the special "Fixed Lens" type. */
function isFixedLensMount(name: string): boolean {
	return name === 'Fixed Lens';
}

/** Build mount dropdown options grouped by format: 35mm, Medium Format, Large Format, Other. */
export function buildMountOptions(mounts: LensMount[], emptyLabel = 'Select mount...'): SelectOption[] {
	const groups: { label: string; mounts: LensMount[] }[] = [
		{ label: '35mm', mounts: [] },
		{ label: 'Medium Format', mounts: [] },
		{ label: 'Large Format', mounts: [] },
		{ label: 'Other', mounts: [] }
	];

	for (const m of mounts) {
		if (isLargeFormatMount(m.name)) {
			groups[2].mounts.push(m);
		} else if (isMediumFormatMount(m.name)) {
			groups[1].mounts.push(m);
		} else if (isFixedLensMount(m.name)) {
			groups[3].mounts.push(m);
		} else {
			groups[0].mounts.push(m);
		}
	}

	const options: SelectOption[] = [{ value: '', label: emptyLabel }];
	for (const group of groups) {
		if (group.mounts.length === 0) continue;
		options.push({ value: '__divider__', label: `── ${group.label} ──`, disabled: true });
		for (const m of group.mounts) {
			options.push({ value: String(m.id), label: m.name });
		}
	}

	options.push({ value: '__divider_new__', label: '── New ──', disabled: true });
	options.push({ value: NEW_MOUNT_OPTION, label: '+ New mount…' });

	return options;
}

export function buildLensLabels(lenses: Lens[]): Map<number, string> {
	return buildDisambiguatedLabels(lenses, lensDisplayName);
}

/** Build lens dropdown options sorted by mount compatibility with the selected camera. */
export function buildLensOptions(
	allLenses: Lens[],
	selectedCamera: { lens_mount_id?: number } | null | undefined,
	emptyLabel = 'No default lens',
	lensMounts: LensMount[] = []
): { value: string; label: string; disabled?: boolean }[] {
	const owned = allLenses.filter((l) => !l.date_sold);
	const labels = buildLensLabels(owned);
	const getLabel = (l: Lens) => labels.get(l.id) ?? lensDisplayName(l);
	const options: { value: string; label: string; disabled?: boolean }[] = [{ value: '', label: emptyLabel }];

	if (selectedCamera?.lens_mount_id) {
		const lfFamily = buildLfFamilyIds(lensMounts);
		const cameraIsLf = lfFamily.has(selectedCamera.lens_mount_id);

		const isCompatible = (lens: Lens) =>
			lens.lens_mount_id === selectedCamera.lens_mount_id || (cameraIsLf && lfFamily.has(lens.lens_mount_id));

		const matching = owned.filter(isCompatible);
		const rest = owned.filter((l) => !isCompatible(l));
		for (const l of matching) options.push({ value: String(l.id), label: getLabel(l) });
		if (matching.length > 0 && rest.length > 0) {
			options.push({ value: '__divider__', label: '── Other lenses ──', disabled: true });
		}
		for (const l of rest) options.push({ value: String(l.id), label: getLabel(l) });
	} else {
		for (const l of owned) options.push({ value: String(l.id), label: getLabel(l) });
	}

	return options;
}
