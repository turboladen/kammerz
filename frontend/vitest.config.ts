import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

// Standalone vitest config — deliberately NOT the SvelteKit vite.config.ts. The
// in-scope unit tests cover pure-logic utils under src/lib/utils, which need
// nothing from the sveltekit() plugin (routing/SSR) and run faster without it.
// Only the `$lib` alias is required (status.ts imports $lib/status-flows.json;
// the other utils use type-only $lib imports that the transform erases).
export default defineConfig({
	resolve: {
		alias: {
			$lib: fileURLToPath(new URL('./src/lib', import.meta.url))
		}
	},
	test: {
		// node env is enough — these utils touch no DOM. Scope discovery to src/ so
		// vitest never tries to run the Playwright e2e specs under tests/ (its
		// default glob would otherwise match tests/smoke.spec.ts).
		include: ['src/**/*.{test,spec}.ts'],
		coverage: {
			provider: 'v8',
			// Report on the logic under test, not the whole app (components/routes
			// have no unit tests yet — including them would just report 0% noise).
			include: ['src/lib/utils/**/*.ts'],
			reporter: ['text', 'html'],
			reportsDirectory: './coverage'
		}
	}
});
