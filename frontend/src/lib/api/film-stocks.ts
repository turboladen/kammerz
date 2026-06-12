import type { FilmStock, FilmStockInsert } from '$lib/types';
import { request } from './client';

export const listFilmStocks = () => request<FilmStock[]>('GET', '/api/film-stocks');
export const getFilmStock = (id: number) => request<FilmStock | null>('GET', `/api/film-stocks/${id}`);
export const createFilmStock = (data: FilmStockInsert) => request<number>('POST', '/api/film-stocks', data);
export const updateFilmStock = (id: number, data: Partial<FilmStockInsert>) =>
	request<void>('PUT', `/api/film-stocks/${id}`, data);
export const deleteFilmStock = (id: number) => request<void>('DELETE', `/api/film-stocks/${id}`);

// --- Distinct value helpers ---

export const listDistinctFilmBrands = () => request<string[]>('GET', '/api/film-stocks/distinct/brands');
