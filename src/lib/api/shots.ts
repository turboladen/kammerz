import { invoke } from '@tauri-apps/api/core';
import type { Shot, ShotInsert } from '$lib/types';

export const listShotsForRoll = (rollId: number) =>
	invoke<Shot[]>('list_shots_for_roll', { rollId });

export const getShot = (id: number) => invoke<Shot | null>('get_shot', { id });

export const createShot = (data: ShotInsert & { lens_ids?: number[] }) =>
	invoke<number>('create_shot', { data });

export const updateShot = (id: number, data: Partial<ShotInsert> & { lens_ids?: number[] }) =>
	invoke<void>('update_shot', { id, data });

export const deleteShot = (id: number) => invoke<void>('delete_shot', { id });

export const getLensesForShot = (shotId: number) =>
	invoke<number[]>('get_lenses_for_shot', { shotId });

export const getLensesForRollShots = (rollId: number) =>
	invoke<[number, number][]>('get_lenses_for_roll_shots', { rollId });

export const suggestNextFrame = (rollId: number) =>
	invoke<string>('suggest_next_frame', { rollId });

export const countShotsForRoll = (rollId: number) =>
	invoke<number>('count_shots_for_roll', { rollId });
