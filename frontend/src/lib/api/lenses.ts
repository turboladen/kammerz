import type { Lens, LensInsert } from '$lib/types';
import { request } from './client';

export const listLenses = () => request<Lens[]>('GET', '/api/lenses');
export const getLens = (id: number) => request<Lens | null>('GET', `/api/lenses/${id}`);
export const createLens = (data: LensInsert) => request<number>('POST', '/api/lenses', data);
export const updateLens = (id: number, data: Partial<LensInsert>) => request<void>('PUT', `/api/lenses/${id}`, data);
export const deleteLens = (id: number) => request<void>('DELETE', `/api/lenses/${id}`);

// --- Distinct value helpers ---

export const listDistinctLensBrands = () => request<string[]>('GET', '/api/lenses/distinct/brands');

// --- Camera associations (reverse lookup) ---

export const getCamerasForLens = (lensId: number) => request<number[]>('GET', `/api/lenses/${lensId}/cameras`);
