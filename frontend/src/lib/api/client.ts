export interface ApiErrorShape {
	code: string;
	message: string;
	status: number;
}

export class ApiRequestError extends Error {
	code: string;
	status: number;
	constructor(e: ApiErrorShape) {
		super(e.message);
		this.name = 'ApiRequestError';
		this.code = e.code;
		this.status = e.status;
	}
}

let onUnauthorized: (() => void) | null = null;
export function setUnauthorizedHandler(fn: (() => void) | null) {
	onUnauthorized = fn;
}

export async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
	const init: RequestInit = { method, credentials: 'include', headers: {} };
	if (body !== undefined) {
		(init.headers as Record<string, string>)['Content-Type'] = 'application/json';
		init.body = JSON.stringify(body);
	}
	let res: Response;
	try {
		res = await fetch(path, init);
	} catch (err) {
		throw new ApiRequestError({
			code: 'NETWORK',
			message: err instanceof Error ? err.message : 'Network error',
			status: 0
		});
	}
	if (res.status === 401 && onUnauthorized) onUnauthorized();
	if (!res.ok) {
		let shape: ApiErrorShape;
		try {
			const j = await res.json();
			shape = {
				code: j.error?.code ?? 'UNKNOWN',
				message: j.error?.message ?? res.statusText,
				status: res.status
			};
		} catch {
			shape = { code: 'UNKNOWN', message: res.statusText, status: res.status };
		}
		throw new ApiRequestError(shape);
	}
	if (res.status === 204) return undefined as T;
	return res.json() as Promise<T>;
}

// Query-string helper for endpoints that take params.
export function qs(params: Record<string, string | number | undefined>): string {
	const p = new URLSearchParams();
	for (const [k, v] of Object.entries(params)) if (v !== undefined) p.set(k, String(v));
	const s = p.toString();
	return s ? `?${s}` : '';
}
