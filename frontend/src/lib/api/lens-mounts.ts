import type { LensMount } from '$lib/types';
import { request } from './client';

export const listLensMounts = () => request<LensMount[]>('GET', '/api/lens-mounts');

export const createLensMount = (name: string) =>
	request<number>('POST', '/api/lens-mounts', { name });
