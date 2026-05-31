import type { CatalogStats } from '$lib/types';
import { request } from './client';

export const getCatalogStats = () => request<CatalogStats>('GET', '/api/stats');
