import { test, expect } from '@playwright/test';
import { BASE, PASSWORD } from './shared';

/**
 * Login-flow tests exercise the real login form, so they must start
 * UNauthenticated — opt out of the project's shared storageState (set by the
 * `setup` project, see auth.setup.ts) back to a clean state.
 *
 * RATE-LIMIT BUDGET: each test here does one real POST /api/auth/login, plus one
 * from the setup project. The backend throttles login per IP and the whole suite
 * shares one IP (LOGIN_BURST_SIZE = 5 in src/auth/rate_limit.rs). We currently
 * use 4 of that budget — keep new real-login tests rare, or they'll re-trip the
 * 429 throttle this storageState setup exists to avoid.
 */
test.describe('login flow', () => {
	test.use({ storageState: { cookies: [], origins: [] } });

	test('login ignores a cross-origin next and stays same-origin', async ({ page }) => {
		// safeNext() must reject a protocol-relative ?next: a successful login may not
		// navigate off-origin. It should fall back to '/' on this origin.
		await page.goto(`${BASE}/login?next=${encodeURIComponent('//attacker.test/')}`);
		await page.fill('input[type=password]', PASSWORD);
		await page.click('button:has-text("Sign in")');
		await page.waitForLoadState('networkidle');
		const url = new URL(page.url());
		expect(url.origin).toBe(new URL(BASE).origin);
		expect(url.pathname).toBe('/');
	});

	test('login gate redirects then admits with correct password', async ({ page }) => {
		await page.goto(`${BASE}/cameras`);
		await expect(page).toHaveURL(/\/login/);
		await page.fill('input[type=password]', PASSWORD);
		await page.click('button:has-text("Sign in")');
		await expect(page).toHaveURL(/\/cameras/);
		await expect(page.locator('body')).toContainText(/camera/i);
	});

	test('login is rejected with a wrong password', async ({ page }) => {
		await page.goto(`${BASE}/cameras`);
		await expect(page).toHaveURL(/\/login/);
		await page.fill('input[type=password]', 'definitely-not-the-password');
		await page.click('button:has-text("Sign in")');
		await expect(page.locator('body')).toContainText(/incorrect password/i);
		await expect(page).toHaveURL(/\/login/);
	});
});

/**
 * Parity smoke: navigate to every main section (already authenticated via the
 * shared storageState) and assert it renders its expected heading without
 * (a) uncaught client errors / console errors and (b) any failed (>=400) /api
 * request. These are the real route dirs under frontend/src/routes/(app)/.
 * There is no standalone /settings or /lens-mounts page — settings (API key)
 * lives inside /import.
 */
const sections: { path: string; heading: RegExp }[] = [
	{ path: '/', heading: /dashboard/i },
	{ path: '/cameras', heading: /cameras/i },
	{ path: '/lenses', heading: /lenses/i },
	{ path: '/film-stocks', heading: /film stocks/i },
	{ path: '/labs', heading: /labs/i },
	{ path: '/rolls', heading: /rolls/i },
	{ path: '/developments', heading: /developments/i },
	{ path: '/search', heading: /search/i },
	{ path: '/stats', heading: /statistics/i },
	{ path: '/quick-entry', heading: /quick entry/i },
	{ path: '/import', heading: /import notes/i }
];

for (const { path, heading } of sections) {
	test(`section ${path} renders without client/api errors`, async ({ page }) => {
		const consoleErrors: string[] = [];
		const pageErrors: string[] = [];
		const apiFailures: string[] = [];

		page.on('console', (msg) => {
			if (msg.type() === 'error') consoleErrors.push(msg.text());
		});
		page.on('pageerror', (err) => pageErrors.push(err.message));
		page.on('response', (res) => {
			const url = res.url();
			if (url.includes('/api/') && res.status() >= 400) {
				apiFailures.push(`${res.status()} ${res.request().method()} ${url}`);
			}
		});

		// Already authenticated via storageState, so this loads the section
		// directly instead of bouncing through /login.
		await page.goto(`${BASE}${path}`);

		// Heading from PageHeader (<h1 class="font-display">) or dashboard hero.
		await expect(page.locator('h1').first()).toContainText(heading);
		// Let any deferred /api calls settle.
		await page.waitForLoadState('networkidle');

		expect(apiFailures, `failed /api requests on ${path}`).toEqual([]);
		expect(pageErrors, `uncaught page errors on ${path}`).toEqual([]);
		expect(consoleErrors, `console errors on ${path}`).toEqual([]);
	});
}

/**
 * Regression guard for kammerz-8k5: the roll-detail page-load $effect must fetch
 * each catalog + the roll's /detail a BOUNDED number of times. The bug was an
 * effect that tracked `roll` (via loadRollData's prevStatus snapshot) and then
 * rewrote it post-fetch, looping forever — dozens of /api/rolls/{id}/detail hits
 * per second. The section loop above visits /rolls (the list) but never a detail
 * page, so it couldn't catch this. We create a throwaway roll (the e2e seed has
 * none), open it, and assert /detail is requested no more than a couple of times.
 */
test('roll detail page loads without an infinite fetch loop (kammerz-8k5)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-LOOP-${Date.now()}`, status: 'loaded' }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const id: number = await created.json();

	let detailCount = 0;
	const consoleErrors: string[] = [];
	page.on('request', (req) => {
		if (req.url().includes(`/api/rolls/${id}/detail`)) detailCount++;
	});
	page.on('console', (msg) => {
		if (msg.type() === 'error') consoleErrors.push(msg.text());
	});

	await page.goto(`${BASE}/rolls/${id}`);
	await expect(page.locator('h1').first()).toContainText(`E2E-LOOP-`);
	await page.waitForLoadState('networkidle');
	// A loop keeps firing past networkidle — give it a fixed window to manifest.
	await page.waitForTimeout(1500);

	expect(detailCount, 'roll /detail should be fetched a bounded number of times, not looped').toBeLessThanOrEqual(2);
	expect(consoleErrors, 'console errors on roll detail').toEqual([]);

	// Tidy up the throwaway roll so it can't perturb other assertions.
	await page.request.delete(`${BASE}/api/rolls/${id}`);
});

/**
 * Smoke test for kammerz-3hq: the two-pane roll detail redesign. The old Timeline
 * section (kammerz-fxl) is replaced by an activity journal; the chevron status
 * control, QuickAddBar, FrameStrip, and RollActivity components replace the former
 * stacked Status + Timeline + Shots sections. Assert the new UI structure is present
 * and the founding "Roll loaded" activity entry appears.
 */
test('roll detail shows status control, frame strip, quick-add, and activity (kammerz-3hq)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-P2-${Date.now()}`, status: 'loaded', frame_count: 36 }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const id: number = await created.json();

	await page.goto(`${BASE}/rolls/${id}`);
	await page.waitForLoadState('networkidle');

	// RollStatusControl renders an h2 with text "Status" (ledger-line header).
	await expect(page.getByRole('heading', { name: 'Status' })).toBeVisible();

	// At least one chevron button is present. For a `loaded` roll the current rung
	// aria-label is "Current status: Loaded"; other rungs say "Move to …".
	await expect(page.getByRole('button', { name: /Current status:|Move to/i }).first()).toBeVisible();

	// Activity journal shows the founding "Roll loaded" entry (RollActivity.svelte
	// renders it as a <span class="text-xs text-text-muted">Roll loaded</span>).
	await expect(page.getByText('Roll loaded')).toBeVisible();

	// QuickAddBar is present: it renders a "Frame" label above the frame number
	// display and a "Save & Next" primary button.
	await expect(page.getByRole('button', { name: /Save & Next/i })).toBeVisible();

	await page.request.delete(`${BASE}/api/rolls/${id}`);
});

/**
 * Regression guard for kammerz-b21: an uncaught route error must render the
 * themed root +error.svelte (status + headline + a way back), not SvelteKit's
 * bare unstyled "Internal Error" fallback. An unmatched route is the simplest
 * uncaught error to trigger; it resolves at the same root boundary that covers
 * the print summary route's +page@ layout reset. (We do NOT assert zero console
 * errors here: a 404 navigation inherently logs SvelteKit's own "Not found" plus
 * the unmatched-resource fetch — those are expected, not page defects.)
 */
test('unmatched route renders the themed error boundary (kammerz-b21)', async ({ page }) => {
	// The release binary serves the SPA fallback (HTTP 200 + index) for unmatched
	// routes, so the client router — not the HTTP status — produces the 404. Assert
	// the client-rendered themed boundary, which holds regardless of transport.
	await page.goto(`${BASE}/this-route-does-not-exist`);
	await expect(page).toHaveTitle(/404 — Kammerz/);
	await expect(page.locator('h1')).toContainText(/page not found/i);
	// The themed boundary offers a way back; the bare fallback does not.
	await expect(page.getByRole('link', { name: /back to dashboard/i })).toBeVisible();
});

/**
 * Regression guard for kammerz-1by: the app ships a real favicon set. The browser's
 * automatic /favicon.ico request used to 404 (no favicon existed at all). Assert the
 * three icon assets serve and that app.html advertises them via <link rel="icon">.
 */
test('favicon assets are served and linked (kammerz-1by)', async ({ page }) => {
	for (const asset of ['/favicon.ico', '/favicon.svg', '/apple-touch-icon.png']) {
		const res = await page.request.get(`${BASE}${asset}`);
		expect(res.ok(), `${asset} should serve 2xx, got ${res.status()}`).toBeTruthy();
		// Not just 2xx: guard the subtler regression where serve_spa's route-like
		// fallback (src/spa.rs) returns 200 + index.html for an asset path. Only the
		// '.' in the filename keeps it out of that fallback — assert a real image.
		const ctype = res.headers()['content-type'] ?? '';
		expect(ctype, `${asset} should be an image, got '${ctype}'`).toContain('image');
	}

	await page.goto(`${BASE}/`);
	// app.html advertises the icon set (SVG preferred, .ico legacy/root fallback).
	// Match hrefs with an ends-with selector: SvelteKit's hydration reconciles
	// <head> and resolves these hrefs to absolute URLs, so a literal "/favicon.svg"
	// attribute match is racy (0 after hydration, 1 before).
	await expect(page.locator('link[rel="icon"][href$="/favicon.svg"]')).toHaveCount(1);
	await expect(page.locator('link[rel="apple-touch-icon"][href$="/apple-touch-icon.png"]')).toHaveCount(1);
});
