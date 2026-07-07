import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
	plugins: [svelte()],
	clearScreen: false,
	server: {
		port: 5173,
		strictPort: true,
		host: host || false,
		watch: {
			ignored: ['**/src-tauri/**']
		}
	}
}));
