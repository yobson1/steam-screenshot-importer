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
		padding-top: 0;
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
