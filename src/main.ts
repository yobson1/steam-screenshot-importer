import { mount } from 'svelte';
import App from './App.svelte';
import { darkModeEnabled } from './stores.svelte';
import runUpdateCheck from './updater';
import { screenshotSettings } from './settings.store.svelte';

function getPreferredTheme() {
	const storedTheme = localStorage.getItem('theme');

	if (storedTheme === 'light' || storedTheme === 'dark') {
		return storedTheme;
	}

	return window.matchMedia?.('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

darkModeEnabled.value = getPreferredTheme() === 'dark';

if (screenshotSettings.checkUpdatesOnStartup) {
	runUpdateCheck();
}

mount(App, { target: document.body });
