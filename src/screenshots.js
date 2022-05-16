import { invoke } from "@tauri-apps/api/tauri";

function importScreenshots(paths, appID) {
	return invoke("import_screenshots", { filePaths: paths, appId: appID });
}

export { importScreenshots };
