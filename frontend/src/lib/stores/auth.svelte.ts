import { goto } from '$app/navigation';
import { getAuthStatus, login as apiLogin, logout as apiLogout } from '$lib/api/auth';
import { setUnauthorizedHandler } from '$lib/api/client';

let authenticated = $state(false);
let authRequired = $state(true);
let initialized = $state(false);

// A 401 from any data call means the session expired mid-session. Flip state and
// bounce to /login with a `next` so the user lands back where they were. Guard
// against a loop when the 401 is the login attempt itself (already on /login).
setUnauthorizedHandler(() => {
	authenticated = false;
	if (typeof window !== 'undefined' && !window.location.pathname.startsWith('/login')) {
		const next = window.location.pathname + window.location.search;
		void goto(`/login?next=${encodeURIComponent(next)}`, { invalidateAll: true });
	}
});

export const auth = {
	get authenticated() {
		return authenticated;
	},
	get authRequired() {
		return authRequired;
	},
	get initialized() {
		return initialized;
	},
	async init() {
		const s = await getAuthStatus();
		authenticated = s.authenticated;
		authRequired = s.auth_required;
		initialized = true;
	},
	async login(password: string) {
		const r = await apiLogin(password);
		authenticated = r.authenticated;
		return r.authenticated;
	},
	async logout() {
		await apiLogout();
		authenticated = false;
	}
};
