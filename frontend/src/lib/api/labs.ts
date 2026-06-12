import type { Lab, LabInsert } from '$lib/types';
import { request } from './client';

export const listLabs = () => request<Lab[]>('GET', '/api/labs');
export const getLab = (id: number) => request<Lab | null>('GET', `/api/labs/${id}`);
export const createLab = (data: LabInsert) => request<number>('POST', '/api/labs', data);
export const updateLab = (id: number, data: Partial<LabInsert>) => request<void>('PUT', `/api/labs/${id}`, data);
export const deleteLab = (id: number) => request<void>('DELETE', `/api/labs/${id}`);
