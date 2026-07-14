import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { exampleFixturesPlugin } from './vite/example-fixtures';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async ({ mode }) => {
	return {
		plugins: [svelte(), exampleFixturesPlugin(mode === 'example')],
		clearScreen: false,
		server: {
			port: 5173,
			strictPort: true,
			host: host || false,
			watch: {
				ignored: ['**/src-tauri/**', 'pkg/**']
			}
		}
	};
});
