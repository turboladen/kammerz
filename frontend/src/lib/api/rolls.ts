import type { RollDetail, RollInsert, RollWithDetails } from '$lib/types';
import { request } from './client';

export const listRolls = () => request<RollWithDetails[]>('GET', '/api/rolls');
export const getRoll = (id: number) => request<RollWithDetails | null>('GET', `/api/rolls/${id}`);
export const getRollDetail = (id: number) => request<RollDetail>('GET', `/api/rolls/${id}/detail`);
export const createRoll = (data: RollInsert) => request<number>('POST', '/api/rolls', data);
export const updateRoll = (id: number, data: Partial<RollInsert>) =>
	request<void>('PUT', `/api/rolls/${id}`, data);
export const deleteRoll = (id: number) => request<void>('DELETE', `/api/rolls/${id}`);

export const listRollsForCamera = (cameraId: number) =>
	request<RollWithDetails[]>('GET', `/api/rolls/for-camera/${cameraId}`);

export const suggestRollId = () => request<string>('GET', '/api/rolls/suggest-id');
