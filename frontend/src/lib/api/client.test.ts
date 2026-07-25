import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiRequestError, qs, request, setUnauthorizedHandler } from './client';

// A minimal stand-in for the parts of `Response` that request() actually reads:
// `.status`, `.ok` (request checks res.ok directly — a plain object won't derive
// it from status, so it MUST be set explicitly), `.statusText`, and `.json()`.
function fakeResponse(opts: {
	status: number;
	ok?: boolean;
	statusText?: string;
	json?: () => Promise<unknown>;
}): Response {
	return {
		status: opts.status,
		ok: opts.ok ?? (opts.status >= 200 && opts.status < 300),
		statusText: opts.statusText ?? '',
		json: opts.json ?? (async () => ({}))
	} as unknown as Response;
}

// The exact fetch signature request() uses, so `mock.calls[0]` is a typed
// `[path, init]` tuple under svelte-check's strict TS (an untyped vi.fn infers
// zero params and makes the tuple indexing an error).
type FetchFn = (path: string, init: RequestInit) => Promise<Response>;

afterEach(() => {
	// The unauthorized handler is a module-global — reset it so a case that
	// registers one can't leak into the next. Also drop the fetch stub.
	setUnauthorizedHandler(null);
	vi.unstubAllGlobals();
});

describe('qs', () => {
	it('skips undefined params', () => {
		expect(qs({ a: 'x', b: undefined })).toBe('?a=x');
	});

	it('encodes values and coerces numbers to strings', () => {
		expect(qs({ q: 'a b&c', n: 5 })).toBe('?q=a+b%26c&n=5');
	});

	it('returns an empty string when nothing is set', () => {
		expect(qs({})).toBe('');
		expect(qs({ a: undefined })).toBe('');
	});
});

describe('request', () => {
	it('resolves the parsed JSON body on success and sends credentials', async () => {
		const fetchMock = vi.fn<FetchFn>(async () => fakeResponse({ status: 200, json: async () => ({ id: 1 }) }));
		vi.stubGlobal('fetch', fetchMock);

		await expect(request('GET', '/api/x')).resolves.toEqual({ id: 1 });

		expect(fetchMock).toHaveBeenCalledTimes(1);
		const [path, init] = fetchMock.mock.calls[0];
		expect(path).toBe('/api/x');
		expect(init.credentials).toBe('include');
	});

	it('sends a JSON content-type and stringified body when a body is given', async () => {
		const fetchMock = vi.fn<FetchFn>(async () => fakeResponse({ status: 200, json: async () => ({}) }));
		vi.stubGlobal('fetch', fetchMock);

		await request('POST', '/api/x', { a: 1 });

		const init = fetchMock.mock.calls[0][1];
		expect(init.method).toBe('POST');
		expect((init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
		expect(init.body).toBe(JSON.stringify({ a: 1 }));
	});

	it('omits the content-type header and body when no body is given', async () => {
		const fetchMock = vi.fn<FetchFn>(async () => fakeResponse({ status: 200, json: async () => ({}) }));
		vi.stubGlobal('fetch', fetchMock);

		await request('GET', '/api/x');

		const init = fetchMock.mock.calls[0][1];
		expect((init.headers as Record<string, string>)['Content-Type']).toBeUndefined();
		expect(init.body).toBeUndefined();
	});

	it('returns undefined for a 204 No Content response', async () => {
		const fetchMock = vi.fn<FetchFn>(async () => fakeResponse({ status: 204 }));
		vi.stubGlobal('fetch', fetchMock);

		await expect(request('DELETE', '/api/x')).resolves.toBeUndefined();
	});

	it('invokes the unauthorized handler AND still throws on a 401', async () => {
		const onUnauthorized = vi.fn();
		setUnauthorizedHandler(onUnauthorized);
		const fetchMock = vi.fn<FetchFn>(async () =>
			fakeResponse({
				status: 401,
				ok: false,
				json: async () => ({ error: { code: 'UNAUTHORIZED', message: 'nope' } })
			})
		);
		vi.stubGlobal('fetch', fetchMock);

		await expect(request('GET', '/api/x')).rejects.toBeInstanceOf(ApiRequestError);
		expect(onUnauthorized).toHaveBeenCalledTimes(1);
	});

	it('does not crash on a 401 when no handler is registered (still throws)', async () => {
		const fetchMock = vi.fn<FetchFn>(async () =>
			fakeResponse({
				status: 401,
				ok: false,
				json: async () => ({ error: { code: 'UNAUTHORIZED', message: 'nope' } })
			})
		);
		vi.stubGlobal('fetch', fetchMock);

		await expect(request('GET', '/api/x')).rejects.toBeInstanceOf(ApiRequestError);
	});

	it('does NOT invoke the unauthorized handler on a non-401 error', async () => {
		// Pins the "only 401 triggers the handler" contract — a regression that
		// broadened the trigger to every non-2xx would fire the login redirect on
		// ordinary 4xx/5xx errors.
		const onUnauthorized = vi.fn();
		setUnauthorizedHandler(onUnauthorized);
		const fetchMock = vi.fn<FetchFn>(async () =>
			fakeResponse({
				status: 500,
				ok: false,
				statusText: 'Internal Server Error',
				json: async () => ({ error: { code: 'INTERNAL', message: 'boom' } })
			})
		);
		vi.stubGlobal('fetch', fetchMock);

		await expect(request('GET', '/api/x')).rejects.toBeInstanceOf(ApiRequestError);
		expect(onUnauthorized).not.toHaveBeenCalled();
	});

	it('throws ApiRequestError carrying the {error:{code,message}} envelope on a non-2xx', async () => {
		const fetchMock = vi.fn<FetchFn>(async () =>
			fakeResponse({
				status: 422,
				ok: false,
				statusText: 'Unprocessable Entity',
				json: async () => ({ error: { code: 'VALIDATION', message: 'bad' } })
			})
		);
		vi.stubGlobal('fetch', fetchMock);

		await expect(request('POST', '/api/x', {})).rejects.toMatchObject({
			code: 'VALIDATION',
			message: 'bad',
			status: 422
		});
	});

	it('falls back to UNKNOWN + statusText when the error body is not JSON', async () => {
		const fetchMock = vi.fn<FetchFn>(async () =>
			fakeResponse({
				status: 500,
				ok: false,
				statusText: 'Internal Server Error',
				json: async () => {
					throw new SyntaxError('Unexpected token');
				}
			})
		);
		vi.stubGlobal('fetch', fetchMock);

		await expect(request('GET', '/api/x')).rejects.toMatchObject({
			code: 'UNKNOWN',
			message: 'Internal Server Error',
			status: 500
		});
	});

	it('throws a NETWORK-shaped ApiRequestError when fetch itself rejects', async () => {
		const fetchMock = vi.fn<FetchFn>(async () => {
			throw new TypeError('Failed to fetch');
		});
		vi.stubGlobal('fetch', fetchMock);

		await expect(request('GET', '/api/x')).rejects.toMatchObject({
			code: 'NETWORK',
			message: 'Failed to fetch',
			status: 0
		});
	});
});
