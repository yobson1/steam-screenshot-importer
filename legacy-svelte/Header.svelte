<script lang="ts">
	import DarkToggle from './DarkToggle.svelte';
	import { Hamburger } from 'svelte-hamburgers';
	import Menu from './Menu.svelte';

	let open = $state(false);
</script>

<header>
	<nav class="left">
		<Menu bind:open />
		<Hamburger bind:open --color="var(--icon-color)" type="arrowalt" />
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
		pointer-events: none;
	}

	nav > :global(*) {
		pointer-events: auto;
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
