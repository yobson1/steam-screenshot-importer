import { invoke } from '@tauri-apps/api/core';
import swal from 'sweetalert';
import { listen } from '@tauri-apps/api/event';

function sendScreenshots(paths: string[] | string, appID: number) {
	return invoke('import_screenshots', { filePaths: paths, appId: appID });
}

let progress = 0;

listen('screenshotImportProgress', (event) => {
	progress = typeof event.payload === 'number' ? event.payload : progress;

	swal({
		title: 'Importing Screenshots',
		text: `${Math.floor(progress)}%`,
		icon: 'info',
		closeOnClickOutside: false,
		closeOnEsc: false,
		buttons: [false, false],
		content: {
			element: 'progress',
			attributes: {
				value: progress,
				max: 100
			}
		}
	});

	if (progress >= 100) {
		progress = 0;
	}
});

listen('screenshotImportError', (event) => {
	swal({
		title: 'Import Error',
		text: `${event.payload}`,
		icon: 'error',
		closeOnClickOutside: false,
		closeOnEsc: false,
		buttons: [false, false]
	});
});

function importScreenshots(appID) {
	invoke('pick_screenshot_files').then((files: string[]) => {
		if (files !== null && files.length > 0) {
			swal({
				title: 'Importing Screenshots',
				text: 'Loading...',
				icon: 'info',
				closeOnClickOutside: false,
				closeOnEsc: false,
				buttons: [false, false]
			});
			sendScreenshots(files, appID).then((err: string) => {
				if (err) {
					swal({
						title: 'Error',
						text: err,
						icon: 'error'
					});

					console.error(err);
				} else {
					swal({
						title: 'Success',
						text: 'Screenshots imported',
						icon: 'success',
						timer: 5000
					});
				}
			});
		} else {
			swal({
				title: 'Error',
				text: 'No files selected',
				icon: 'error'
			});
		}
	});
}

export { importScreenshots };
