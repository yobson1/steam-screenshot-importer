<script lang="ts">
	import { darkModeEnabled } from './stores.svelte';

	function toggle() {
		darkModeEnabled.setTheme(darkModeEnabled.value ? 'light' : 'dark');
	}
</script>

<button
	type="button"
	onclick={toggle}
	aria-label="Toggle dark mode"
	aria-pressed={darkModeEnabled.value}
>
	<span aria-hidden="true"></span>
</button>

<style>
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
