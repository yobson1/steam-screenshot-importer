import { spawnSync } from 'node:child_process';

if ('NO_STEAMWORKS' in process.env) {
	console.log('NO_STEAMWORKS set, skipping Steamworks binary fetch');
	process.exit(0);
}

const result = spawnSync(
	'cargo',
	['run', '--manifest-path', 'src-tauri/tools/steamworks-fetcher/Cargo.toml'],
	{
		stdio: 'inherit'
	}
);

if (result.error) {
	console.error(result.error);
	process.exit(1);
}

process.exit(result.status ?? 1);
