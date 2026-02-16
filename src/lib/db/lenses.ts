import { getDb } from './index';
import type { Lens, LensInsert } from '$lib/types';

export async function listLenses(): Promise<Lens[]> {
	const db = await getDb();
	return db.select('SELECT * FROM lenses ORDER BY brand, name_on_lens');
}

export async function getLens(id: number): Promise<Lens | undefined> {
	const db = await getDb();
	const rows: Lens[] = await db.select('SELECT * FROM lenses WHERE id = $1', [id]);
	return rows[0];
}

export async function createLens(lens: LensInsert): Promise<number> {
	const db = await getDb();
	const result = await db.execute(
		`INSERT INTO lenses (brand, lens_system, name_on_lens, focal_length, max_aperture, min_aperture, filter_thread_front_mm, filter_thread_rear_mm, serial_number, date_purchased, purchased_from, date_sold, notes)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)`,
		[
			lens.brand,
			lens.lens_system,
			lens.name_on_lens,
			lens.focal_length,
			lens.max_aperture,
			lens.min_aperture,
			lens.filter_thread_front_mm,
			lens.filter_thread_rear_mm,
			lens.serial_number,
			lens.date_purchased,
			lens.purchased_from,
			lens.date_sold,
			lens.notes
		]
	);
	return result.lastInsertId;
}

export async function updateLens(id: number, lens: Partial<LensInsert>): Promise<void> {
	const db = await getDb();
	const fields: string[] = [];
	const values: unknown[] = [];
	let idx = 1;

	const entries = Object.entries(lens).filter(([_, v]) => v !== undefined);
	for (const [key, value] of entries) {
		fields.push(`${key} = $${idx}`);
		values.push(value);
		idx++;
	}

	if (fields.length === 0) return;

	fields.push(`updated_at = datetime('now')`);
	values.push(id);

	await db.execute(`UPDATE lenses SET ${fields.join(', ')} WHERE id = $${idx}`, values);
}

export async function deleteLens(id: number): Promise<void> {
	const db = await getDb();
	await db.execute('DELETE FROM lenses WHERE id = $1', [id]);
}

// --- Distinct value helpers (for ComboInput autocomplete) ---

export async function listDistinctLensBrands(): Promise<string[]> {
	const db = await getDb();
	const rows: { brand: string }[] = await db.select('SELECT DISTINCT brand FROM lenses ORDER BY brand');
	return rows.map((r) => r.brand);
}

export async function listDistinctLensSystems(): Promise<string[]> {
	const db = await getDb();
	const rows: { lens_system: string }[] = await db.select(
		'SELECT DISTINCT lens_system FROM lenses WHERE lens_system IS NOT NULL ORDER BY lens_system'
	);
	return rows.map((r) => r.lens_system);
}

export async function getCamerasForLens(lensId: number): Promise<number[]> {
	const db = await getDb();
	const rows: { camera_id: number }[] = await db.select(
		'SELECT camera_id FROM camera_lenses WHERE lens_id = $1',
		[lensId]
	);
	return rows.map((r) => r.camera_id);
}
