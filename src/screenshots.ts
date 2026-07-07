import { invoke } from '@tauri-apps/api/core';
import Swal from 'sweetalert2';
import { listen } from '@tauri-apps/api/event';

function sendScreenshots(paths: string[] | string, appID: number) {
	return invoke<string>('import_screenshots', { filePaths: paths, appId: appID });
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

listen('screenshotImportError', (event) => {
	Swal.fire({
		title: 'Import Error',
		text: `${event.payload}`,
		icon: 'error',
		allowOutsideClick: false,
		allowEscapeKey: false
	});
});

function importScreenshots(appID: number) {
	invoke<string[]>('pick_screenshot_files').then((files) => {
		if (files !== null && files.length > 0) {
			Swal.fire({
				title: 'Importing Screenshots',
				text: 'Loading...',
				showConfirmButton: false,
				allowOutsideClick: false,
				allowEscapeKey: false
			});

			sendScreenshots(files, appID).then((err) => {
				if (err) {
					Swal.fire({
						title: 'Error',
						text: err,
						icon: 'error'
					});

					console.error(err);
				} else {
					Swal.fire({
						title: 'Success',
						text: 'Screenshots imported',
						icon: 'success',
						timer: 5000,
						showConfirmButton: false
					});
				}
			});
		} else {
			Swal.fire({
				title: 'Error',
				text: 'No files selected',
				icon: 'error'
			});
		}
	});
}

export { importScreenshots };
