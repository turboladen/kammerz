/**
 * Resolve a post-login `next` target to a safe same-origin path, or `/` when it
 * is missing, malformed, or an open-redirect vector.
 *
 * Resolves `raw` against `origin` and accepts ONLY same-origin results — this
 * rejects protocol-relative (`//host`), backslash (`/\host`), and control-char
 * (`/\t/host`) forms that the URL parser would otherwise resolve cross-origin,
 * which a plain prefix check misses. A `next=/login` is also rejected so login
 * never bounces back to itself.
 *
 * Pure (no `$app/state`) so it is unit-testable; the login page passes
 * `page.url.searchParams.get('next')` and `page.url.origin` (kammerz-vlyu.11).
 */
export function safeNext(raw: string | null, origin: string): string {
	if (!raw) return '/';
	try {
		const url = new URL(raw, origin);
		if (url.origin !== origin) return '/';
		// Never bounce back to the login form (e.g. a stray ?next=/login).
		if (url.pathname === '/login') return '/';
		return url.pathname + url.search + url.hash;
	} catch {
		return '/';
	}
}
