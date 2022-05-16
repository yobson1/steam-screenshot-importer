import App from "./App.svelte";
import { darkModeEnabled } from "./stores.js";

function setDark(dark: boolean) {
	if (dark) {
		window.document.body.classList.add("dark-mode");
	} else {
		window.document.body.classList.remove("dark-mode");
	}
	darkModeEnabled.set(dark);
	localStorage.setItem("theme", dark ? "dark" : "light");
}

// Auto-detect dark/light theme choice
if (localStorage.getItem("theme") === "dark" || (window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches)) {
	document.body.classList.remove("dark-mode");
	setDark(true);
}

const app = new App({
	target: document.body,
});

export default app;
