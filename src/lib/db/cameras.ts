import { getDb } from './index';
import type { Camera, CameraInsert, CameraMaintenance, CameraMaintenanceInsert } from '$lib/types';

export async function listCameras(): Promise<Camera[]> {
	const db = await getDb();
	return db.select('SELECT * FROM cameras ORDER BY brand, model');
}

export async function getCamera(id: number): Promise<Camera | undefined> {
	const db = await getDb();
	const rows: Camera[] = await db.select('SELECT * FROM cameras WHERE id = $1', [id]);
	return rows[0];
}

export async function createCamera(camera: CameraInsert): Promise<number> {
	const db = await getDb();
	const result = await db.execute(
		`INSERT INTO cameras (brand, model, prefix, format, camera_type, serial_number, date_purchased, purchased_from, date_sold, notes)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)`,
		[
			camera.brand,
			camera.model,
			camera.prefix,
			camera.format,
			camera.camera_type,
			camera.serial_number,
			camera.date_purchased,
			camera.purchased_from,
			camera.date_sold,
			camera.notes
		]
	);
	return result.lastInsertId;
}

export async function updateCamera(id: number, camera: Partial<CameraInsert>): Promise<void> {
	const db = await getDb();
	const fields: string[] = [];
	const values: unknown[] = [];
	let idx = 1;

	const entries = Object.entries(camera).filter(([_, v]) => v !== undefined);
	for (const [key, value] of entries) {
		fields.push(`${key} = $${idx}`);
		values.push(value);
		idx++;
	}

	if (fields.length === 0) return;

	fields.push(`updated_at = datetime('now')`);
	values.push(id);

	await db.execute(`UPDATE cameras SET ${fields.join(', ')} WHERE id = $${idx}`, values);
}

export async function deleteCamera(id: number): Promise<void> {
	const db = await getDb();
	await db.execute('DELETE FROM cameras WHERE id = $1', [id]);
}

// --- Maintenance Records ---

export async function listMaintenanceForCamera(cameraId: number): Promise<CameraMaintenance[]> {
	const db = await getDb();
	return db.select(
		'SELECT * FROM camera_maintenance WHERE camera_id = $1 ORDER BY date_done DESC',
		[cameraId]
	);
}

export async function createMaintenance(record: CameraMaintenanceInsert): Promise<number> {
	const db = await getDb();
	const result = await db.execute(
		`INSERT INTO camera_maintenance (camera_id, maintenance_type, done_by, date_done, cost, notes)
		 VALUES ($1, $2, $3, $4, $5, $6)`,
		[
			record.camera_id,
			record.maintenance_type,
			record.done_by,
			record.date_done,
			record.cost,
			record.notes
		]
	);
	return result.lastInsertId;
}

export async function updateMaintenance(
	id: number,
	record: Partial<CameraMaintenanceInsert>
): Promise<void> {
	const db = await getDb();
	const fields: string[] = [];
	const values: unknown[] = [];
	let idx = 1;

	const entries = Object.entries(record).filter(([_, v]) => v !== undefined);
	for (const [key, value] of entries) {
		fields.push(`${key} = $${idx}`);
		values.push(value);
		idx++;
	}

	if (fields.length === 0) return;

	fields.push(`updated_at = datetime('now')`);
	values.push(id);

	await db.execute(
		`UPDATE camera_maintenance SET ${fields.join(', ')} WHERE id = $${idx}`,
		values
	);
}

export async function deleteMaintenance(id: number): Promise<void> {
	const db = await getDb();
	await db.execute('DELETE FROM camera_maintenance WHERE id = $1', [id]);
}

// --- Camera-Lens Associations ---

export async function getLensesForCamera(cameraId: number): Promise<number[]> {
	const db = await getDb();
	const rows: { lens_id: number }[] = await db.select(
		'SELECT lens_id FROM camera_lenses WHERE camera_id = $1',
		[cameraId]
	);
	return rows.map((r) => r.lens_id);
}

export async function linkLensToCamera(cameraId: number, lensId: number): Promise<void> {
	const db = await getDb();
	await db.execute(
		'INSERT OR IGNORE INTO camera_lenses (camera_id, lens_id) VALUES ($1, $2)',
		[cameraId, lensId]
	);
}

export async function unlinkLensFromCamera(cameraId: number, lensId: number): Promise<void> {
	const db = await getDb();
	await db.execute(
		'DELETE FROM camera_lenses WHERE camera_id = $1 AND lens_id = $2',
		[cameraId, lensId]
	);
}

// --- Distinct value helpers (for ComboInput autocomplete) ---

export async function listDistinctCameraBrands(): Promise<string[]> {
	const db = await getDb();
	const rows: { brand: string }[] = await db.select('SELECT DISTINCT brand FROM cameras ORDER BY brand');
	return rows.map((r) => r.brand);
}

export async function listDistinctVendors(): Promise<string[]> {
	const db = await getDb();
	const rows: { purchased_from: string }[] = await db.select(
		`SELECT DISTINCT purchased_from FROM cameras WHERE purchased_from IS NOT NULL
		 UNION
		 SELECT DISTINCT purchased_from FROM lenses WHERE purchased_from IS NOT NULL
		 ORDER BY 1`
	);
	return rows.map((r) => r.purchased_from);
}

export async function listDistinctMaintProviders(): Promise<string[]> {
	const db = await getDb();
	const rows: { done_by: string }[] = await db.select(
		'SELECT DISTINCT done_by FROM camera_maintenance WHERE done_by IS NOT NULL ORDER BY done_by'
	);
	return rows.map((r) => r.done_by);
}
