import { checkUpdate, installUpdate } from '@tauri-apps/api/updater';
import { relaunch } from '@tauri-apps/api/process';
import { listen } from '@tauri-apps/api/event';
import swal from 'sweetalert';

// Handle auto-updates
listen('tauri://update-status', (event) => {
	console.log('status', event);
});

let totalDownloaded = 0;
listen('tauri://update-download-progress', (event) => {
	totalDownloaded += event.payload.chunkLength;
	let percentage = Math.floor((totalDownloaded / event.payload.contentLength) * 100);
	swal({
		title: 'Downloading update',
		text: percentage < 100 ? `${percentage}%` : 'Restarting',
		icon: 'info',
		buttons: false,
		closeOnClickOutside: false,
		closeOnEsc: false,
		content: {
			element: 'progress',
			attributes: {
				value: percentage,
				max: 100
			}
		}
	});
});

async function runUpdateCheck() {
	const { shouldUpdate, manifest } = await checkUpdate();
	if (shouldUpdate) {
		if (
			await swal({
				title: 'Update available',
				text: `Update to version ${manifest.version} available:\n${manifest.body}`,
				icon: 'info',
				buttons: ['Nope', 'Update'],
				closeOnClickOutside: false
			})
		) {
			swal({
				title: 'Downloading update',
				icon: 'info',
				text: '0%',
				buttons: false,
				closeOnClickOutside: false,
				closeOnEsc: false,
				content: {
					element: 'progress',
					attributes: {
						value: 0,
						max: 100
					}
				}
			});
			await installUpdate();
			await relaunch();
		}
	}
}

export default runUpdateCheck;
