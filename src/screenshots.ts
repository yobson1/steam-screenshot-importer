import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";
import { pictureDir } from "@tauri-apps/api/path";
import swal from "sweetalert";
import { listen } from "@tauri-apps/api/event";

function sendScreenshots(paths: string[] | string, appID: number) {
	return invoke("import_screenshots", { filePaths: paths, appId: appID });
}

listen("screenshotImportProgress", event => {
	swal({
		title: "Importing Screenshots",
		text: `${event.payload}`,
		icon: "info",
		closeOnClickOutside: false,
		closeOnEsc: false,
		buttons: [false, false],
	});
});

listen("screenshotImportError", event => {
	swal({
		title: "Import Error",
		text: `${event.payload}`,
		icon: "error",
		closeOnClickOutside: false,
		closeOnEsc: false,
		buttons: [false, false],
	});
});

function importScreenshots(appID: number) {
	pictureDir().then((dir) => {
		// https://github.com/image-rs/image#supported-image-formats
		open({
			defaultPath: dir,
			filters: [
				{
					name: "Images",
					extensions: [
						"png",
						"jpg",
						"jpeg",
						"bmp",
						"ico",
						"tiff",
						"tif",
						"webp",
						"avif",
						"pnm",
						"dds",
						"tga",
						"exr",
					],
				},
			],
			multiple: true,
			title: "Select screenshots to import",
		}).then((files) => {
			if (files !== null && files.length > 0) {
				swal({
					title: "Importing Screenshots",
					text: "Loading...",
					icon: "info",
					closeOnClickOutside: false,
					closeOnEsc: false,
					buttons: [false, false],
				});
				sendScreenshots(files, appID).then((err: string) => {
					if (err) {
						swal({
							title: "Error",
							text: err,
							icon: "error",
						});

						console.error(err);
					} else {
						swal({
							title: "Success",
							text: "Screenshots imported",
							icon: "success",
							timer: 5000,
						});
					}
				});
			} else {
				swal({
					title: "Error",
					text: "No files selected",
					icon: "error",
				});
			}
		});
	});
}

export { importScreenshots };
