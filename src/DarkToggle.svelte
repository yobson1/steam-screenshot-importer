<script>
	import { darkModeEnabled } from "./stores.js";

	function toggle() {
		window.document.body.classList.toggle("dark-mode");
		let isDark = window.document.body.classList.contains("dark-mode");
		darkModeEnabled.set(isDark);
		localStorage.setItem("theme", isDark ? "dark" : "light");
	}
</script>

<button on:click={toggle} />

<style>
	button {
		background-image: url("/moon.svg");
		background-size: cover;
		border: none;
		cursor: pointer;
		height: 2rem;
		width: 2rem;
		background-color: transparent;
		color: transparent;
		transition: background-color var(--transition-speed);
		margin: 15px;
	}

	button:hover {
		background-color: rgba(0, 0, 0, 0.5);
	}

	:global(body.dark-mode) button {
		background-image: url("/sun.svg");
		filter: invert();
	}
</style>
