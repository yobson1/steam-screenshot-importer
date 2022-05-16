<script>
	import DarkToggle from "./DarkToggle.svelte";
	import { Hamburger } from "svelte-hamburgers";
	import Menu from "./Menu.svelte";
	import { darkModeEnabled } from "./stores.js";

	let open;
</script>

<header>
	<nav class="left">
		<Menu bind:open />
		<Hamburger
			bind:open
			--color={$darkModeEnabled ? "white" : "black"}
			type="arrowalt"
		/>
	</nav>
	<nav class="right">
		<DarkToggle />
	</nav>
</header>

<style>
	header {
		display: flex;
		position: fixed;
		top: 0.5rem;
		z-index: 1;
		width: calc(100% - (var(--body-padding) * 2));
	}

	nav {
		margin: 0;
		margin-bottom: 0.5rem;
		padding: 0;
		width: 50%;
		display: flex;
	}

	nav.left {
		justify-content: flex-start;
	}

	nav.right {
		justify-content: flex-end;
		z-index: 2;
	}
</style>
