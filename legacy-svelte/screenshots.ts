import Swal from 'sweetalert2';
import { listen } from '@tauri-apps/api/event';
import { mount, unmount } from 'svelte';
import { screenshotSettings } from './settings.store.svelte';
import { commands, type ImportError, type ImportFailure } from './bindings';
import ImportErrorList from './ImportErrorList.svelte';

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

function isImportError(error: unknown): error is ImportError {
	if (error === null || typeof error !== 'object') return false;

	const candidate = error as Partial<ImportError>;
	return (
		typeof candidate.summary === 'string' &&
		Array.isArray(candidate.errors) &&
		candidate.errors.every(
			(item) =>
				item !== null &&
				typeof item === 'object' &&
				typeof item.filePath === 'string' &&
				typeof item.message === 'string'
		)
	);
}

async function showErrorDetails(errors: ImportFailure[]) {
	const container = document.createElement('div');
	const component = mount(ImportErrorList, {
		target: container,
		props: { errors }
	});

	try {
		await Swal.fire({
			title: `Import errors (${errors.length})`,
			html: container,
			width: 'min(48rem, calc(100vw - 2rem))',
			confirmButtonText: 'Close',
			customClass: {
				htmlContainer: 'import-errors-container'
			}
		});
	} finally {
		await unmount(component);
	}
}

async function showImportError(error: unknown) {
	const importError: ImportError = isImportError(error)
		? error
		: { summary: errorMessage(error), errors: [] };
	const hasDetails = importError.errors.length > 0;

	console.error(importError.summary, ...importError.errors);

	const result = await Swal.fire({
		title: 'Errors occurred',
		text: importError.summary,
		icon: 'error',
		confirmButtonText: hasDetails ? 'View errors' : 'Close',
		showCancelButton: hasDetails,
		cancelButtonText: 'Close'
	});

	if (!hasDetails || !result.isConfirmed) return;

	await showErrorDetails(importError.errors);
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
		await showImportError(error);
	}
}

export { importScreenshots };
