import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const host = process.env.TAURI_DEV_HOST;
const exampleFixtureDir = fileURLToPath(new URL('./screenshots/fixtures', import.meta.url));
const fixturePattern = /^\d+\.(?:jpe?g|png|webp)$/i;

function getExampleFixtures() {
	return readdirSync(exampleFixtureDir)
		.filter((filename) => fixturePattern.test(filename))
		.sort();
}

function exampleFixturesPlugin(filenames: string[]): Plugin {
	return {
		name: 'example-screenshot-fixtures',
		configureServer(server) {
			server.middlewares.use((request, response, next) => {
				const requestedFilename = decodeURIComponent(request.url || '').match(
					/^\/fixtures\/([^?]+)(?:\?.*)?$/
				)?.[1];

				if (!requestedFilename || !filenames.includes(requestedFilename)) {
					next();
					return;
				}

				response.setHeader('Content-Type', 'image/jpeg');
				response.end(readFileSync(path.join(exampleFixtureDir, requestedFilename)));
			});
		},
		generateBundle() {
			for (const filename of filenames) {
				this.emitFile({
					type: 'asset',
					fileName: `fixtures/${filename}`,
					source: readFileSync(path.join(exampleFixtureDir, filename))
				});
			}
		}
	};
}

export default defineConfig(async ({ mode }) => {
	const exampleFixtures = mode === 'example' ? getExampleFixtures() : [];

	return {
		plugins: [svelte(), ...(mode === 'example' ? [exampleFixturesPlugin(exampleFixtures)] : [])],
		define: {
			__EXAMPLE_FIXTURES__: JSON.stringify(exampleFixtures)
		},
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
