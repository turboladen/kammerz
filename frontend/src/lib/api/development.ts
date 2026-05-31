import type {
	DevelopmentLab,
	DevelopmentLabInsert,
	DevelopmentSelf,
	DevelopmentSelfInsert,
	DevStage,
	DevStageInsert,
	SelfDevListItem
} from '$lib/types';
import { request } from './client';

// --- Lab Development ---

export const getLabDevForRoll = (rollId: number) =>
	request<DevelopmentLab | null>('GET', `/api/development/lab/for-roll/${rollId}`);

export const createLabDev = (data: DevelopmentLabInsert) =>
	request<number>('POST', '/api/development/lab', data);

export const updateLabDev = (id: number, data: Partial<DevelopmentLabInsert>) =>
	request<void>('PUT', `/api/development/lab/${id}`, data);

export const deleteLabDev = (id: number) => request<void>('DELETE', `/api/development/lab/${id}`);

// --- Self Development ---

export const getSelfDevForRoll = (rollId: number) =>
	request<DevelopmentSelf | null>('GET', `/api/development/self/for-roll/${rollId}`);

export const createSelfDev = (data: DevelopmentSelfInsert & { stages?: DevStageInsert[] }) =>
	request<number>('POST', '/api/development/self', data);

export const updateSelfDev = (
	id: number,
	data: Partial<DevelopmentSelfInsert> & { stages?: DevStageInsert[] }
) => request<void>('PUT', `/api/development/self/${id}`, data);

export const deleteSelfDev = (id: number) => request<void>('DELETE', `/api/development/self/${id}`);

// --- Dev Stages ---

export const listDevStages = (developmentSelfId: number) =>
	request<DevStage[]>('GET', `/api/development/self/${developmentSelfId}/stages`);

// --- List all self-developments ---

export const listAllSelfDevelopments = () =>
	request<SelfDevListItem[]>('GET', '/api/development/self');
