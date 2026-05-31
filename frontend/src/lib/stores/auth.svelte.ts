import { getAuthStatus, login as apiLogin, logout as apiLogout } from '$lib/api/auth';
import { setUnauthorizedHandler } from '$lib/api/client';

let authenticated = $state(false);
let authRequired = $state(true);
let initialized = $state(false);

setUnauthorizedHandler(() => {
	authenticated = false;
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
