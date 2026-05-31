import { invoke } from '@tauri-apps/api/core';
import type { SearchResults } from '$lib/types';

export const searchCatalog = (query: string) =>
	invoke<SearchResults>('search_catalog', { query });
