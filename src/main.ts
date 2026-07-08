import { mount } from 'svelte';
import App from './App.svelte';
import { syncThemeWithDocument } from './stores.svelte';
import runUpdateCheck from './updater';
import { screenshotSettings } from './settings.store.svelte';

syncThemeWithDocument();

if (screenshotSettings.checkUpdatesOnStartup) {
	runUpdateCheck();
}

mount(App, { target: document.body });
