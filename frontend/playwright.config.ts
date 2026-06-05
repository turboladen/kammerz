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
		// retries: 0 means every test runs once and is never a "retry", so
		// 'on-first-retry' would never capture anything. Key off the outcome
		// instead so a failed single attempt writes a trace.zip + screenshot.
		trace: 'retain-on-failure',
		screenshot: 'only-on-failure'
	},
	projects: [
		{
			name: 'chromium',
			use: { ...devices['Desktop Chrome'] }
		}
	]
});
