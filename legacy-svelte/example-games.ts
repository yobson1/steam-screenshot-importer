import type { Game } from './bindings';

export const exampleGames: Game[] = __EXAMPLE_FIXTURES__
	.map((filename) => {
		const appId = Number(filename.match(/^(\d+)\.[^.]+$/)?.[1]);

		if (!Number.isInteger(appId)) {
			throw new Error(`Screenshot fixture must be named with its numeric app ID: ${filename}`);
		}

		return {
			appId,
			appName: `App ${appId}`,
			imageSrc: `/fixtures/${filename}`
		};
	})
	.sort((a, b) => a.appId - b.appId);
