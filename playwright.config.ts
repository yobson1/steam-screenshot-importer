import { defineConfig } from '@playwright/test';
import path from 'node:path';

export default defineConfig({
	testDir: './tests',
	fullyParallel: false,
	workers: 1,
	reporter: 'line',
	outputDir: path.join(process.env.RUNNER_TEMP || '/tmp', 'steam-screenshot-importer-playwright'),
	use: {
		baseURL: 'http://127.0.0.1:4173',
		viewport: { width: 1388, height: 780 }
	},
	webServer: {
		command: 'pnpm preview --host 127.0.0.1',
		url: 'http://127.0.0.1:4173',
		reuseExistingServer: !process.env.CI
	}
});
