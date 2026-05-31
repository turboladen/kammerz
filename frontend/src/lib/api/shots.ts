import type { Shot, ShotInsert } from '$lib/types';
import { request } from './client';

export const listShotsForRoll = (rollId: number) =>
	request<Shot[]>('GET', `/api/shots/for-roll/${rollId}`);

export const getShot = (id: number) => request<Shot | null>('GET', `/api/shots/${id}`);

export const createShot = (data: ShotInsert & { lens_ids?: number[] }) =>
	request<number>('POST', '/api/shots', data);

export const updateShot = (id: number, data: Partial<ShotInsert> & { lens_ids?: number[] }) =>
	request<void>('PUT', `/api/shots/${id}`, data);

export const deleteShot = (id: number) => request<void>('DELETE', `/api/shots/${id}`);

export const getLensesForShot = (shotId: number) =>
	request<number[]>('GET', `/api/shots/${shotId}/lenses`);

export const getLensesForRollShots = (rollId: number) =>
	request<[number, number][]>('GET', `/api/shots/for-roll/${rollId}/lenses`);

export const suggestNextFrame = (rollId: number) =>
	request<string>('GET', `/api/shots/for-roll/${rollId}/next-frame`);

export const countShotsForRoll = (rollId: number) =>
	request<number>('GET', `/api/shots/for-roll/${rollId}/count`);
