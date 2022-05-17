import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";
import { pictureDir } from "@tauri-apps/api/path";
import swal from "sweetalert";

function sendScreenshots(paths: string[] | string, appID: number) {
	return invoke("import_screenshots", { filePaths: paths, appId: appID });
}

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
				sendScreenshots(files, appID).then((err: string) => {
					if (err) {
						swal({
							title: "Error",
							text: err,
							icon: "error",
						});

						console.error(err);
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
