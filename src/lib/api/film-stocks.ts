import { invoke } from '@tauri-apps/api/core';
import type { FilmStock, FilmStockInsert } from '$lib/types';

export const listFilmStocks = () => invoke<FilmStock[]>('list_film_stocks');

export const getFilmStock = (id: number) => invoke<FilmStock | null>('get_film_stock', { id });

export const createFilmStock = (data: FilmStockInsert) =>
	invoke<number>('create_film_stock', { data });

export const updateFilmStock = (id: number, data: Partial<FilmStockInsert>) =>
	invoke<void>('update_film_stock', { id, data });

export const deleteFilmStock = (id: number) => invoke<void>('delete_film_stock', { id });

// --- Distinct value helpers ---

export const listDistinctFilmBrands = () => invoke<string[]>('list_distinct_film_brands');
