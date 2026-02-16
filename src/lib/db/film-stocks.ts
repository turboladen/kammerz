import { getDb } from './index';
import type { FilmStock, FilmStockInsert } from '$lib/types';

export async function listFilmStocks(): Promise<FilmStock[]> {
	const db = await getDb();
	return db.select('SELECT * FROM film_stocks ORDER BY brand, name, format');
}

export async function getFilmStock(id: number): Promise<FilmStock | undefined> {
	const db = await getDb();
	const rows: FilmStock[] = await db.select('SELECT * FROM film_stocks WHERE id = $1', [id]);
	return rows[0];
}

export async function createFilmStock(stock: FilmStockInsert): Promise<number> {
	const db = await getDb();
	const result = await db.execute(
		`INSERT INTO film_stocks (brand, name, format, exposure_count, stock_type, iso, notes)
		 VALUES ($1, $2, $3, $4, $5, $6, $7)`,
		[stock.brand, stock.name, stock.format, stock.exposure_count, stock.stock_type, stock.iso, stock.notes]
	);
	return result.lastInsertId;
}

export async function updateFilmStock(id: number, stock: Partial<FilmStockInsert>): Promise<void> {
	const db = await getDb();
	const fields: string[] = [];
	const values: unknown[] = [];
	let idx = 1;

	const entries = Object.entries(stock).filter(([_, v]) => v !== undefined);
	for (const [key, value] of entries) {
		fields.push(`${key} = $${idx}`);
		values.push(value);
		idx++;
	}

	if (fields.length === 0) return;

	fields.push(`updated_at = datetime('now')`);
	values.push(id);

	await db.execute(`UPDATE film_stocks SET ${fields.join(', ')} WHERE id = $${idx}`, values);
}

export async function deleteFilmStock(id: number): Promise<void> {
	const db = await getDb();
	await db.execute('DELETE FROM film_stocks WHERE id = $1', [id]);
}

// --- Distinct value helpers (for ComboInput autocomplete) ---

export async function listDistinctFilmBrands(): Promise<string[]> {
	const db = await getDb();
	const rows: { brand: string }[] = await db.select('SELECT DISTINCT brand FROM film_stocks ORDER BY brand');
	return rows.map((r) => r.brand);
}
