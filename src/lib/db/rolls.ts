import { getDb } from './index';
import type { Roll, RollInsert, RollWithDetails } from '$lib/types';

export async function listRolls(): Promise<RollWithDetails[]> {
	const db = await getDb();
	return db.select(`
		SELECT r.*,
			c.brand AS camera_brand,
			c.model AS camera_model,
			fs.brand AS film_stock_brand,
			fs.name AS film_stock_name,
			fs.iso AS film_stock_iso
		FROM rolls r
		LEFT JOIN cameras c ON r.camera_id = c.id
		LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id
		ORDER BY r.created_at DESC
	`);
}

export async function getRoll(id: number): Promise<RollWithDetails | undefined> {
	const db = await getDb();
	const rows: RollWithDetails[] = await db.select(
		`SELECT r.*,
			c.brand AS camera_brand,
			c.model AS camera_model,
			fs.brand AS film_stock_brand,
			fs.name AS film_stock_name,
			fs.iso AS film_stock_iso
		FROM rolls r
		LEFT JOIN cameras c ON r.camera_id = c.id
		LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id
		WHERE r.id = $1`,
		[id]
	);
	return rows[0];
}

export async function createRoll(roll: RollInsert): Promise<number> {
	const db = await getDb();
	const result = await db.execute(
		`INSERT INTO rolls (roll_id, camera_id, film_stock_id, status, frame_count, date_loaded, date_finished, date_fuzzy, push_pull, notes)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
		[
			roll.roll_id,
			roll.camera_id,
			roll.film_stock_id,
			roll.status,
			roll.frame_count,
			roll.date_loaded,
			roll.date_finished,
			roll.date_fuzzy,
			roll.push_pull,
			roll.notes
		]
	);
	return result.lastInsertId;
}

export async function updateRoll(id: number, roll: Partial<RollInsert>): Promise<void> {
	const db = await getDb();
	const fields: string[] = [];
	const values: unknown[] = [];
	let idx = 1;

	const entries = Object.entries(roll).filter(([_, v]) => v !== undefined);
	for (const [key, value] of entries) {
		fields.push(`${key} = $${idx}`);
		values.push(value);
		idx++;
	}

	if (fields.length === 0) return;

	fields.push(`updated_at = datetime('now')`);
	values.push(id);

	await db.execute(`UPDATE rolls SET ${fields.join(', ')} WHERE id = $${idx}`, values);
}

export async function deleteRoll(id: number): Promise<void> {
	const db = await getDb();
	await db.execute('DELETE FROM rolls WHERE id = $1', [id]);
}

export async function listRollsForCamera(cameraId: number): Promise<RollWithDetails[]> {
	const db = await getDb();
	return db.select(
		`SELECT r.*,
			c.brand AS camera_brand,
			c.model AS camera_model,
			fs.brand AS film_stock_brand,
			fs.name AS film_stock_name,
			fs.iso AS film_stock_iso
		FROM rolls r
		LEFT JOIN cameras c ON r.camera_id = c.id
		LEFT JOIN film_stocks fs ON r.film_stock_id = fs.id
		WHERE r.camera_id = $1
		ORDER BY r.created_at DESC`,
		[cameraId]
	);
}

export async function suggestRollId(): Promise<string> {
	const db = await getDb();
	const now = new Date();
	const prefix =
		String(now.getFullYear()).slice(2) +
		String(now.getMonth() + 1).padStart(2, '0') +
		String(now.getDate()).padStart(2, '0');

	const rows: { count: number }[] = await db.select(
		`SELECT COUNT(*) as count FROM rolls WHERE roll_id LIKE $1`,
		[`${prefix}-%`]
	);

	const next = (rows[0]?.count ?? 0) + 1;
	return `${prefix}-${next}`;
}
