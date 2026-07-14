import { mount } from 'svelte';
import App from './App.svelte';
import { syncThemeWithDocument } from './stores.svelte';
import runUpdateCheck from './updater';
import { screenshotSettings } from './settings.store.svelte';
import { exampleMode } from './example-mode';

syncThemeWithDocument();

if (!exampleMode && screenshotSettings.checkUpdatesOnStartup) {
	runUpdateCheck();
}

mount(App, { target: document.body });
