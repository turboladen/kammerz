import type { Lens } from '$lib/types';

export function lensDisplayName(lens: Lens): string {
	if (lens.name_on_lens) return lens.name_on_lens;
	const parts = [lens.brand];
	if (lens.focal_length) parts.push(`${lens.focal_length}mm`);
	if (lens.max_aperture) parts.push(`f/${lens.max_aperture}`);
	return parts.join(' ');
}
