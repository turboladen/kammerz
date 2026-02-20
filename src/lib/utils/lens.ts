import type { Lens } from '$lib/types';

export function lensDisplayName(lens: Lens): string {
	if (lens.name_on_lens) return lens.name_on_lens;
	const parts = [lens.brand];
	if (lens.focal_length) parts.push(`${lens.focal_length}mm`);
	if (lens.max_aperture) parts.push(`f/${lens.max_aperture}`);
	return parts.join(' ');
}

/** Build lens dropdown options sorted by mount compatibility with the selected camera. */
export function buildLensOptions(
	allLenses: Lens[],
	selectedCamera: { lens_mount_id?: number } | null | undefined,
	emptyLabel = 'No default lens'
): { value: string; label: string; disabled?: boolean }[] {
	const owned = allLenses.filter((l) => !l.date_sold);
	const options: { value: string; label: string; disabled?: boolean }[] = [
		{ value: '', label: emptyLabel }
	];

	if (selectedCamera?.lens_mount_id) {
		const matching = owned.filter((l) => l.lens_mount_id === selectedCamera.lens_mount_id);
		const rest = owned.filter((l) => l.lens_mount_id !== selectedCamera.lens_mount_id);
		for (const l of matching) options.push({ value: String(l.id), label: lensDisplayName(l) });
		if (matching.length > 0 && rest.length > 0) {
			options.push({ value: '__divider__', label: '── Other lenses ──', disabled: true });
		}
		for (const l of rest) options.push({ value: String(l.id), label: lensDisplayName(l) });
	} else {
		for (const l of owned) options.push({ value: String(l.id), label: lensDisplayName(l) });
	}

	return options;
}
