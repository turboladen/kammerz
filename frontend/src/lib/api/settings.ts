import { request } from './client';

export const getSetting = (key: string) =>
	request<string | null>('GET', `/api/settings/${encodeURIComponent(key)}`);

export const setSetting = (key: string, value: string) =>
	request<void>('PUT', `/api/settings/${encodeURIComponent(key)}`, { value });
