import { invoke } from '@tauri-apps/api/core';
import type { RollInsert, RollWithDetails } from '$lib/types';

export const listRolls = () => invoke<RollWithDetails[]>('list_rolls');

export const getRoll = (id: number) => invoke<RollWithDetails | null>('get_roll', { id });

export const createRoll = (data: RollInsert) => invoke<number>('create_roll', { data });

export const updateRoll = (id: number, data: Partial<RollInsert>) =>
	invoke<void>('update_roll', { id, data });

export const deleteRoll = (id: number) => invoke<void>('delete_roll', { id });

export const listRollsForCamera = (cameraId: number) =>
	invoke<RollWithDetails[]>('list_rolls_for_camera', { cameraId });

export const suggestRollId = () => invoke<string>('suggest_roll_id');
