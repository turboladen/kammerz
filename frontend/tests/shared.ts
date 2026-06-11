import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Single source for the e2e env contract — kept in one module so the setup
// project, the section tests, and playwright.config.ts can't drift apart.
export const BASE = process.env.E2E_BASE ?? 'http://localhost:3002';
export const PASSWORD = process.env.E2E_PASSWORD ?? 'secret';

// Saved login storageState. ABSOLUTE on purpose: the `setup` project writes it
// via storageState({ path }) (resolved against cwd) while the `chromium` project
// reads it via a config `storageState` string (resolved against the config dir).
// An absolute path makes both resolve to the same file no matter which cwd
// Playwright is launched from. Mirrors the .gitignore entry frontend/playwright/.auth/.
const here = path.dirname(fileURLToPath(import.meta.url));
export const AUTH_FILE = path.join(here, '..', 'playwright', '.auth', 'user.json');
