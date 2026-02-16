import { invoke } from '@tauri-apps/api/core';
import type { Lab, LabInsert } from '$lib/types';

export const listLabs = () => invoke<Lab[]>('list_labs');

export const getLab = (id: number) => invoke<Lab | null>('get_lab', { id });

export const createLab = (data: LabInsert) => invoke<number>('create_lab', { data });

export const updateLab = (id: number, data: Partial<LabInsert>) =>
	invoke<void>('update_lab', { id, data });

export const deleteLab = (id: number) => invoke<void>('delete_lab', { id });
