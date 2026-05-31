import { auth } from '$lib/stores/auth.svelte';
import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ url }) => {
	if (!auth.initialized) await auth.init();
	if (auth.authRequired && !auth.authenticated) {
		const next = url.pathname + url.search;
		throw redirect(307, `/login?next=${encodeURIComponent(next)}`);
	}
	return {};
};
