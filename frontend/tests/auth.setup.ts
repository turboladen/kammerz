import { test as setup, expect } from '@playwright/test';

// Authenticate ONCE for the whole suite. The section parity tests reuse the
// saved storageState (cookie) instead of logging in per test — that keeps the
// number of POST /api/auth/login requests tiny so the backend's per-IP
// brute-force throttle (tower-governor, all requests from the runner share one
// IP) never trips. Only the login-flow tests in smoke.spec.ts log in for real.
const BASE = process.env.E2E_BASE ?? 'http://localhost:3002';
const PASSWORD = process.env.E2E_PASSWORD ?? 'secret';

export const AUTH_FILE = 'playwright/.auth/user.json';

setup('authenticate', async ({ page }) => {
	await page.goto(`${BASE}/cameras`);
	await expect(page).toHaveURL(/\/login/);
	await page.fill('input[type=password]', PASSWORD);
	await page.click('button:has-text("Sign in")');
	await expect(page).toHaveURL(/\/cameras/);
	await page.context().storageState({ path: AUTH_FILE });
});
