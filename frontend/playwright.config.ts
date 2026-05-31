import { defineConfig, devices } from '@playwright/test';

// E2E smoke/parity tests run against a manually-started release binary
// (`target/release/kammerz`), not a Vite dev server — so there is no
// `webServer` block. Point at the running server via E2E_BASE.
const baseURL = process.env.E2E_BASE ?? 'http://localhost:3002';

export default defineConfig({
	testDir: './tests',
	fullyParallel: false,
	forbidOnly: !!process.env.CI,
	retries: 0,
	workers: 1,
	reporter: 'list',
	use: {
		baseURL,
		trace: 'on-first-retry'
	},
	projects: [
		{
			name: 'chromium',
			use: { ...devices['Desktop Chrome'] }
		}
	]
});
