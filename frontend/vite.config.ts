import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const backendPort = Number(process.env.PORT) || 3002;

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		// 5273, not Vite's default 5173, so kammerz dev doesn't collide with the
		// sibling fewd app (also on 5173). strictPort fails loudly if it's taken
		// rather than silently drifting to another port.
		port: 5273,
		strictPort: true,
		proxy: {
			'/api': { target: `http://localhost:${backendPort}`, changeOrigin: true }
		}
	}
});
