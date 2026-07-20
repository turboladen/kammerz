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
		data: { roll_id: `E2E-LOOP-${Date.now()}` }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const id: number = await created.json();
	// try/finally so a failing assertion can't leak the roll into the shared DB.
	try {
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
	} finally {
		// Tidy up the throwaway roll so it can't perturb other assertions.
		await page.request.delete(`${BASE}/api/rolls/${id}`);
	}
});

/**
 * Smoke test for the activity board (kammerz-64ga, replacing the kammerz-3hq chevron
 * assertions). The chevron status control + Lifecycle-dates section are gone; the
 * roll page now derives five activities (ADR-0013) and renders them as the activity
 * board. In the shooting phase the board is collapsed by default — expanding it
 * reveals the per-activity rows. QuickAddBar + the founding "Roll loaded" journal
 * entry survive.
 */
test('roll detail shows the activity board, quick-add, and activity journal (kammerz-64ga)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-P3-${Date.now()}`, frame_count: 36 }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const id: number = await created.json();
	// try/finally so a failing assertion can't leak the roll into the shared DB.
	try {
		await page.goto(`${BASE}/rolls/${id}`);
		await page.waitForLoadState('networkidle');

		// The board renders a ledger-line "Activity" header. A fresh loaded roll is in
		// the shooting phase, so the board starts collapsed — showing the derived badge
		// ("Loaded") and a "Show details" affordance.
		await expect(page.getByRole('heading', { name: 'Activity', exact: true })).toBeVisible();
		await expect(page.getByRole('button', { name: /Show details/ })).toBeVisible();

		// Expanding surfaces the per-activity rows. Assert labels unique to the board
		// (Development also names the DevelopmentSection heading below, so skip it here).
		await page.getByRole('button', { name: /Show details/ }).click();
		await expect(page.getByText('Shooting', { exact: true })).toBeVisible();
		await expect(page.getByText('Scanning', { exact: true })).toBeVisible();
		await expect(page.getByText('Post-processing', { exact: true })).toBeVisible();
		await expect(page.getByText('Archiving', { exact: true })).toBeVisible();

		// Activity journal shows the founding "Roll loaded" entry (RollActivity.svelte
		// renders it as a <span class="text-xs text-text-muted">Roll loaded</span>).
		await expect(page.getByText('Roll loaded')).toBeVisible();

		// QuickAddBar is present in the shooting phase: a "Save & Next" primary button.
		await expect(page.getByRole('button', { name: /Save & Next/i })).toBeVisible();
	} finally {
		await page.request.delete(`${BASE}/api/rolls/${id}`);
	}
});

/**
 * Board interaction (kammerz-64ga): completing an activity by setting its date from
 * the board must persist to the roll and advance the derived lifecycle. Set the
 * Shooting "Finished" date via the board's date editor, then assert the write landed
 * (date_finished) and the board reflects it.
 */
test('activity board sets a lifecycle date and it persists (kammerz-64ga)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-P3B-${Date.now()}`, frame_count: 36 }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const id: number = await created.json();
	// try/finally so a failing assertion can't leak the roll into the shared DB.
	try {
		await page.goto(`${BASE}/rolls/${id}`);
		await page.waitForLoadState('networkidle');

		// Expand the board (collapsed by default in the shooting phase), then open the
		// Shooting "Finished" date editor and save (DateConfirm seeds today). The
		// accessible name carries the activity ("Set Shooting finished date") because
		// caption-only names collide across activities sharing a caption.
		await page.getByRole('button', { name: /Show details/ }).click();
		await page.getByRole('button', { name: 'Set Shooting finished date' }).click();
		const dialog = page.getByRole('dialog');
		await expect(dialog).toBeVisible();
		await dialog.getByRole('button', { name: 'Save', exact: true }).click();
		await expect(dialog).toBeHidden();
		await page.waitForLoadState('networkidle');

		// The date persisted server-side (date_finished is now set).
		const roll = await (await page.request.get(`${BASE}/api/rolls/${id}`)).json();
		expect(roll.date_finished, 'Shooting completion date must persist').toBeTruthy();
		// And the roll advanced past shooting — the derived badge is no longer "Loaded".
		expect(roll.badge).not.toBe('Loaded');
	} finally {
		await page.request.delete(`${BASE}/api/rolls/${id}`);
	}
});

/**
 * Regression guard for kammerz-11o3: edits made in the Edit Shot dialog must
 * survive < > navigation to an adjacent shot (auto-save-on-navigate). The old
 * behavior re-seeded the shared form fields from the target shot, silently
 * discarding unsaved edits.
 */
test('edit-shot dialog auto-saves edits when navigating between shots (kammerz-11o3)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-NAV-${Date.now()}`, frame_count: 36 }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const rollId: number = await created.json();
	// try/finally so a failing assertion can't leak the roll (and its shots).
	try {
		const shotIds: number[] = [];
		for (const frame of ['1', '2']) {
			const res = await page.request.post(`${BASE}/api/shots`, {
				data: { roll_id: rollId, frame_number: frame, lens_ids: [] }
			});
			expect(res.ok(), `create shot ${frame} failed: ${res.status()}`).toBeTruthy();
			shotIds.push(await res.json());
		}

		await page.goto(`${BASE}/rolls/${rollId}`);
		await page.waitForLoadState('networkidle');

		// Click frame 1 via the FrameStrip — the dialog now opens VIEW-first
		// (kammerz-4she), so there's no form yet. Scope every in-dialog locator through
		// role=dialog — the QuickAddBar behind it also has a <textarea> and a
		// "Save & Next" button.
		await page.getByRole('button', { name: /^Frame 1[ ,].*click to view/ }).click();
		const dialog = page.getByRole('dialog');
		await expect(dialog.getByText('Shot 1 of 2')).toBeVisible();
		await expect(dialog.locator('textarea'), 'view mode shows no form').toHaveCount(0);

		// Enter edit mode via the dialog-scoped Edit button (bare "Edit" collides with
		// the roll-header / board / development Edits). Now the form is present.
		await dialog.getByRole('button', { name: 'Edit', exact: true }).click();

		// Edit shot 1's notes, then navigate — this must auto-save shot 1 (edit-mode
		// nav carries the mode across, so shot 2 opens straight into the form).
		await dialog.locator('textarea').fill('note one');
		await dialog.getByRole('button', { name: 'Next shot' }).click();
		await expect(dialog.getByText('Shot 2 of 2')).toBeVisible();

		// Edit shot 2's notes and save normally. The dialog closes only AFTER the
		// PUT resolves (handleSaveShot awaits updateShot before flipping
		// showShotDialog), so waiting for it to disappear removes the race between
		// the async save and the API assertions below.
		await dialog.locator('textarea').fill('note two');
		await dialog.getByRole('button', { name: 'Save', exact: true }).click();
		await expect(dialog).toBeHidden();

		// Both edits persisted server-side.
		const shot1 = await (await page.request.get(`${BASE}/api/shots/${shotIds[0]}`)).json();
		const shot2 = await (await page.request.get(`${BASE}/api/shots/${shotIds[1]}`)).json();
		expect(shot1.notes, 'shot 1 edit must survive < > navigation').toBe('note one');
		expect(shot2.notes).toBe('note two');
	} finally {
		await page.request.delete(`${BASE}/api/rolls/${rollId}`);
	}
});

/**
 * kammerz-4she: clicking a shot opens the dialog READ-ONLY (view-first) with the
 * shot's fields displayed and an Edit button — the form appears only after Edit.
 */
test('shot dialog opens view-first and Edit reveals the form (kammerz-4she)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-VIEW-${Date.now()}`, frame_count: 36 }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const rollId: number = await created.json();
	try {
		const res = await page.request.post(`${BASE}/api/shots`, {
			data: { roll_id: rollId, frame_number: '1', aperture: '8', notes: 'sunny sixteen', lens_ids: [] }
		});
		expect(res.ok(), `create shot failed: ${res.status()}`).toBeTruthy();

		await page.goto(`${BASE}/rolls/${rollId}`);
		await page.waitForLoadState('networkidle');

		// Click the shot via the FrameStrip. The dialog opens view-first: the read-only
		// value (f/8) is shown and there is NO form textarea yet.
		await page.getByRole('button', { name: /^Frame 1[ ,].*click to view/ }).click();
		const dialog = page.getByRole('dialog');
		// Title is the h2 "Shot 1" (exact — the nav header renders "Shot 1 of 2").
		await expect(dialog.getByRole('heading', { name: 'Shot 1', exact: true })).toBeVisible();
		await expect(dialog.getByText('f/8')).toBeVisible();
		await expect(dialog.locator('textarea'), 'view mode is read-only').toHaveCount(0);

		// Edit reveals the form, seeded with the shot's values.
		await dialog.getByRole('button', { name: 'Edit', exact: true }).click();
		await expect(dialog.locator('textarea')).toHaveValue('sunny sixteen');
	} finally {
		await page.request.delete(`${BASE}/api/rolls/${rollId}`);
	}
});

/**
 * kammerz-4she: the Frames section can toggle to a shots TABLE that lists every
 * shot's fields (frame, f/, shutter, location, …) for zero-click reading, and a
 * table row opens the same view-first dialog.
 */
test('shots table view lists fields and opens the view dialog (kammerz-4she)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-TABLE-${Date.now()}`, frame_count: 36 }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const rollId: number = await created.json();
	try {
		const res = await page.request.post(`${BASE}/api/shots`, {
			data: {
				roll_id: rollId,
				frame_number: '1',
				aperture: '5.6',
				shutter_speed: '1/250',
				location: 'Corlieu Falls',
				lens_ids: []
			}
		});
		expect(res.ok(), `create shot failed: ${res.status()}`).toBeTruthy();

		await page.goto(`${BASE}/rolls/${rollId}`);
		await page.waitForLoadState('networkidle');

		// A fresh loaded roll is in the shooting phase → strip by default. Switch to
		// Table via the segmented control. The control is a native radiogroup with
		// visually-hidden (sr-only) inputs — Playwright's actionability check
		// rejects clicking hidden elements, so click the visible label text, then
		// assert the radio actually became checked.
		await page.getByText('Table', { exact: true }).click();
		await expect(page.getByRole('radio', { name: 'Table', exact: true })).toBeChecked();

		// The table renders the decorated fields (f/ prefix, s suffix) and the location.
		await expect(page.getByRole('cell', { name: 'f/5.6', exact: true })).toBeVisible();
		await expect(page.getByRole('cell', { name: '1/250s', exact: true })).toBeVisible();
		await expect(page.getByRole('cell', { name: 'Corlieu Falls', exact: true })).toBeVisible();

		// The Frame-cell button is the row's control; clicking it opens the view dialog.
		await page.getByRole('button', { name: 'View frame 1', exact: true }).click();
		const dialog = page.getByRole('dialog');
		await expect(dialog.getByRole('heading', { name: 'Shot 1', exact: true })).toBeVisible();
	} finally {
		await page.request.delete(`${BASE}/api/rolls/${rollId}`);
	}
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
	await expect(page.locator('link[rel="icon"][href$="/favicon.ico"]')).toHaveCount(1);
	await expect(page.locator('link[rel="icon"][href$="/favicon.svg"]')).toHaveCount(1);
	await expect(page.locator('link[rel="apple-touch-icon"][href$="/apple-touch-icon.png"]')).toHaveCount(1);
});
