import type { SearchResults } from '$lib/types';
import { qs, request } from './client';

export const searchCatalog = (query: string) =>
	request<SearchResults>('GET', `/api/search${qs({ q: query })}`);
