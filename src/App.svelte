<script lang="ts">
	import Header from './Header.svelte';
	import Footer from './Footer.svelte';
	import Home from './Home.svelte';
	import Settings from './Settings.svelte';
	import About from './About.svelte';
	import Router from 'svelte-spa-router';

	const routes = {
		'/': Home,
		'/settings': Settings,
		'/about': About
	};
</script>

<Header />

<main>
	<Router {routes} />
</main>

<Footer />

<style>
	:global(body) {
		background-color: var(--background-light);
		color: var(--content-light);
		transition:
			background-color var(--transition-speed),
			color var(--transition-speed);
		padding-top: 0;
	}

	:global(body.dark-mode) {
		background-color: var(--background-dark);
		color: var(--content-dark);
	}

	main {
		text-align: center;
	}

	@media (min-width: 640px) {
		main {
			max-width: none;
		}
	}
</style>
