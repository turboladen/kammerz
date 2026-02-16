import { invoke } from '@tauri-apps/api/core';
import type {
	DevelopmentLab,
	DevelopmentLabInsert,
	DevelopmentSelf,
	DevelopmentSelfInsert,
	DevStage,
	DevStageInsert
} from '$lib/types';

// --- Lab Development ---

export const getLabDevForRoll = (rollId: number) =>
	invoke<DevelopmentLab | null>('get_lab_dev_for_roll', { rollId });

export const createLabDev = (data: DevelopmentLabInsert) =>
	invoke<number>('create_lab_dev', { data });

export const updateLabDev = (id: number, data: Partial<DevelopmentLabInsert>) =>
	invoke<void>('update_lab_dev', { id, data });

export const deleteLabDev = (id: number) => invoke<void>('delete_lab_dev', { id });

// --- Self Development ---

export const getSelfDevForRoll = (rollId: number) =>
	invoke<DevelopmentSelf | null>('get_self_dev_for_roll', { rollId });

export const createSelfDev = (data: DevelopmentSelfInsert & { stages?: DevStageInsert[] }) =>
	invoke<number>('create_self_dev', { data });

export const updateSelfDev = (
	id: number,
	data: Partial<DevelopmentSelfInsert> & { stages?: DevStageInsert[] }
) => invoke<void>('update_self_dev', { id, data });

export const deleteSelfDev = (id: number) => invoke<void>('delete_self_dev', { id });

// --- Dev Stages ---

export const listDevStages = (developmentSelfId: number) =>
	invoke<DevStage[]>('list_dev_stages', { developmentSelfId });
