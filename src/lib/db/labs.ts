import { getDb } from './index';
import type { Lab, LabInsert } from '$lib/types';

export async function listLabs(): Promise<Lab[]> {
	const db = await getDb();
	return db.select('SELECT * FROM labs ORDER BY name');
}

export async function getLab(id: number): Promise<Lab | undefined> {
	const db = await getDb();
	const rows: Lab[] = await db.select('SELECT * FROM labs WHERE id = $1', [id]);
	return rows[0];
}

export async function createLab(lab: LabInsert): Promise<number> {
	const db = await getDb();
	const result = await db.execute(
		`INSERT INTO labs (name, location, website, notes) VALUES ($1, $2, $3, $4)`,
		[lab.name, lab.location, lab.website, lab.notes]
	);
	return result.lastInsertId;
}

export async function updateLab(id: number, lab: Partial<LabInsert>): Promise<void> {
	const db = await getDb();
	const fields: string[] = [];
	const values: unknown[] = [];
	let idx = 1;

	const entries = Object.entries(lab).filter(([_, v]) => v !== undefined);
	for (const [key, value] of entries) {
		fields.push(`${key} = $${idx}`);
		values.push(value);
		idx++;
	}

	if (fields.length === 0) return;

	fields.push(`updated_at = datetime('now')`);
	values.push(id);

	await db.execute(`UPDATE labs SET ${fields.join(', ')} WHERE id = $${idx}`, values);
}

export async function deleteLab(id: number): Promise<void> {
	const db = await getDb();
	await db.execute('DELETE FROM labs WHERE id = $1', [id]);
}
