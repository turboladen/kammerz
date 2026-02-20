import { invoke } from '@tauri-apps/api/core';
import type { Lens, LensInsert } from '$lib/types';

export const listLenses = () => invoke<Lens[]>('list_lenses');

export const getLens = (id: number) => invoke<Lens | null>('get_lens', { id });

export const createLens = (data: LensInsert) => invoke<number>('create_lens', { data });

export const updateLens = (id: number, data: Partial<LensInsert>) =>
	invoke<void>('update_lens', { id, data });

export const deleteLens = (id: number) => invoke<void>('delete_lens', { id });

// --- Distinct value helpers ---

export const listDistinctLensBrands = () => invoke<string[]>('list_distinct_lens_brands');

// --- Camera associations (reverse lookup, Phase 4) ---

export const getCamerasForLens = (lensId: number) =>
	invoke<number[]>('get_cameras_for_lens', { lensId });
