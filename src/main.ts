import { mount } from 'svelte';
import App from './App.svelte';
import { darkModeEnabled } from './stores.svelte';
import runUpdateCheck from './updater';
import { screenshotSettings } from './settings.store.svelte';

function setDark(dark: boolean) {
	document.body.classList.toggle('dark-mode', dark);
	darkModeEnabled.value = dark;
	localStorage.setItem('theme', dark ? 'dark' : 'light');
}

if (
	localStorage.getItem('theme') == 'light' ||
	(localStorage.getItem('theme') === null &&
		window.matchMedia?.('(prefers-color-scheme: light)').matches)
) {
	setDark(false);
}

if (screenshotSettings.checkUpdatesOnStartup) {
	runUpdateCheck();
}

mount(App, { target: document.body });
