<script lang="ts">
	import { darkModeEnabled } from './stores.svelte';

	function setTheme(theme: 'light' | 'dark') {
		darkModeEnabled.value = theme === 'dark';
		localStorage.setItem('theme', theme);
	}

	function toggle() {
		setTheme(darkModeEnabled.value ? 'light' : 'dark');
	}
</script>

<div class="theme-toggle">
	<input
		class="theme-light"
		type="radio"
		name="theme"
		checked={!darkModeEnabled.value}
		tabindex="-1"
	/>
	<input
		class="theme-dark"
		type="radio"
		name="theme"
		checked={darkModeEnabled.value}
		tabindex="-1"
	/>
	<button
		type="button"
		onclick={toggle}
		aria-label="Toggle dark mode"
		aria-pressed={darkModeEnabled.value}
	>
		<span aria-hidden="true"></span>
	</button>
</div>

<style>
	.theme-toggle {
		display: contents;
	}

	input {
		display: none;
	}

	button {
		border: none;
		cursor: pointer;
		height: 2rem;
		width: 2rem;
		padding: 0;
		background-color: transparent;
		color: transparent;
		transition: background-color var(--transition-speed);
		margin: 15px;
	}

	button:hover {
		background-color: var(--theme-toggle-hover-background-color);
	}

	span {
		display: block;
		width: 100%;
		height: 100%;
		background-color: var(--icon-color);
		mask: var(--theme-toggle-icon) center / cover no-repeat;
	}
</style>
