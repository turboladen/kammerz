import { test, expect, type Page } from '@playwright/test';

const BASE = process.env.E2E_BASE ?? 'http://localhost:3001';
const PASSWORD = process.env.E2E_PASSWORD ?? 'secret';

/**
 * Log in via the real login form, leaving the page on the post-login target.
 * Visiting an (app) route unauthenticated bounces to /login?next=…; we fill
 * the password, click "Sign in", and the guard admits us to `next`.
 */
async function login(page: Page, next = '/cameras') {
	await page.goto(`${BASE}${next}`);
	await expect(page).toHaveURL(/\/login/);
	await page.fill('input[type=password]', PASSWORD);
	await page.click('button:has-text("Sign in")');
	// We should land back on the requested route — assert the exact pathname so a
	// strand on /login (or a wrong redirect) fails instead of matching loosely.
	await expect(page).toHaveURL((url) => url.pathname === next);
}

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

/**
 * Parity smoke: after logging in, navigate to every main section and assert it
 * renders its expected heading without (a) uncaught client errors / console
 * errors and (b) any failed (>=400) /api request. These are the real route dirs
 * under frontend/src/routes/(app)/. There is no standalone /settings or
 * /lens-mounts page — settings (API key) lives inside /import.
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

		await login(page, path);

		// Heading from PageHeader (<h1 class="font-display">) or dashboard hero.
		await expect(page.locator('h1').first()).toContainText(heading);
		// Let any deferred /api calls settle.
		await page.waitForLoadState('networkidle');

		expect(apiFailures, `failed /api requests on ${path}`).toEqual([]);
		expect(pageErrors, `uncaught page errors on ${path}`).toEqual([]);
		expect(consoleErrors, `console errors on ${path}`).toEqual([]);
	});
}
