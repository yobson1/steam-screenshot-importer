import { Persisted, asEnum } from './persisted.svelte';

const theme = new Persisted(
	'theme',
	(window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? true) ? 'dark' : 'light',
	asEnum(['light', 'dark'] as const)
);

class DarkMode {
	get value(): boolean {
		return theme.value === 'dark';
	}

	setTheme(newTheme: 'light' | 'dark') {
		theme.set(newTheme);
	}
}

export const darkModeEnabled = new DarkMode();

export function syncThemeWithDocument() {
	$effect.root(() => {
		$effect(() => {
			document.documentElement.dataset.theme = darkModeEnabled.value ? 'dark' : 'light';
		});
	});
}
