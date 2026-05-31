import { request } from './client';

export interface AuthStatus {
	authenticated: boolean;
	auth_required: boolean;
}

export const getAuthStatus = () => request<AuthStatus>('GET', '/api/auth/me');

export const login = (password: string) =>
	request<{ authenticated: boolean }>('POST', '/api/auth/login', { password });

export const logout = () => request<{ authenticated: boolean }>('POST', '/api/auth/logout');
