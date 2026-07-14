import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';

const fixtureDir = fileURLToPath(new URL('../screenshots/fixtures', import.meta.url));
const fixturePattern = /^\d+\.(?:jpe?g|png|webp)$/i;

function findFixtures() {
	return readdirSync(fixtureDir)
		.filter((filename) => fixturePattern.test(filename))
		.sort();
}

export function exampleFixturesPlugin(enabled: boolean): Plugin {
	const filenames = enabled ? findFixtures() : [];

	return {
		name: 'example-screenshot-fixtures',
		config() {
			return {
				define: {
					__EXAMPLE_FIXTURES__: JSON.stringify(filenames)
				}
			};
		},
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
				response.end(readFileSync(path.join(fixtureDir, requestedFilename)));
			});
		},
		generateBundle() {
			for (const filename of filenames) {
				this.emitFile({
					type: 'asset',
					fileName: `fixtures/${filename}`,
					source: readFileSync(path.join(fixtureDir, filename))
				});
			}
		}
	};
}
