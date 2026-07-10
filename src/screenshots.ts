import Swal from 'sweetalert2';
import { listen } from '@tauri-apps/api/event';
import { screenshotSettings } from './settings.store.svelte';
import { commands } from './bindings';

function sendScreenshots(paths: string[], appID: number) {
	return commands.importScreenshots(
		paths,
		appID,
		screenshotSettings.jpegQuality,
		screenshotSettings.filterType
	);
}

function errorMessage(error: unknown): string {
	if (typeof error === 'string') return error;
	if (error instanceof Error) return error.message;
	return 'An unexpected error occurred';
}

let progress = 0;

listen('screenshotImportProgress', (event) => {
	progress = typeof event.payload === 'number' ? event.payload : progress;

	if (!Swal.isVisible()) {
		Swal.fire({
			title: 'Importing Screenshots',
			html: `
				<div>${Math.floor(progress)}%</div>
				<progress value="${progress}" max="100"></progress>
			`,
			showConfirmButton: false,
			allowOutsideClick: false,
			allowEscapeKey: false
		});
	} else {
		const container = Swal.getHtmlContainer();

		if (container) {
			container.innerHTML = `
				<div>${Math.floor(progress)}%</div>
				<progress value="${progress}" max="100"></progress>
			`;
		}
	}

	if (progress >= 100) {
		progress = 0;
	}
});

async function importScreenshots(appID: number) {
	try {
		const files = await commands.pickScreenshotFiles();

		if (files.length === 0) {
			await Swal.fire({
				title: 'Error',
				text: 'No files selected',
				icon: 'error'
			});
			return;
		}

		void Swal.fire({
			title: 'Importing Screenshots',
			text: 'Loading...',
			showConfirmButton: false,
			allowOutsideClick: false,
			allowEscapeKey: false
		});

		await sendScreenshots(files, appID);

		await Swal.fire({
			title: 'Success',
			text: 'Screenshots imported',
			icon: 'success',
			timer: 5000,
			showConfirmButton: false
		});
	} catch (error) {
		const message = errorMessage(error);
		console.error(message);

		await Swal.fire({
			title: 'Error',
			text: message,
			icon: 'error'
		});
	}
}

export { importScreenshots };
