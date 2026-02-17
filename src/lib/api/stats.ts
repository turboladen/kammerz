import { invoke } from '@tauri-apps/api/core';
import type { CatalogStats } from '$lib/types';

export const getCatalogStats = () => invoke<CatalogStats>('get_catalog_stats');
