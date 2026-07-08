class DarkMode {
	value = $state(loadPreferredTheme());

	setTheme(theme: 'light' | 'dark') {
		this.value = theme === 'dark';
		localStorage.setItem('theme', theme);
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

function loadPreferredTheme(): boolean {
	const stored = localStorage.getItem('theme');

	if (stored === 'light' || stored === 'dark') {
		return stored === 'dark';
	}

	return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? true;
}
