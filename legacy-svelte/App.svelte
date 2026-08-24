<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import Header from './Header.svelte';
	import Footer from './Footer.svelte';
	import Home from './Home.svelte';
	import Settings from './Settings.svelte';
	import About from './About.svelte';
	import { router } from 'svelte-spa-router';

	type ScrollPosition = { x: number; y: number };

	const scrollPositions = new SvelteMap<string, ScrollPosition>();
	let currentRoute = router.location;
	let navigationId = 0;

	function routeFromUrl(url: string) {
		const hashStart = url.indexOf('#/');
		const route = hashStart === -1 ? '/' : url.slice(hashStart + 1);
		return route.split('?')[0];
	}

	function saveCurrentScrollPosition() {
		scrollPositions.set(currentRoute, { x: window.scrollX, y: window.scrollY });
	}

	onMount(() => {
		const previousScrollRestoration = history.scrollRestoration;
		history.scrollRestoration = 'manual';
		saveCurrentScrollPosition();

		function handleNavigationClick(event: MouseEvent) {
			const target = event.target instanceof Element ? event.target.closest('a') : null;
			const href = target?.getAttribute('href');
			if (!href?.startsWith('#/')) return;

			saveCurrentScrollPosition();
		}

		async function handleHashChange(event: HashChangeEvent) {
			const toRoute = event.newURL ? routeFromUrl(event.newURL) : routeFromUrl(location.href);
			const thisNavigation = ++navigationId;

			currentRoute = toRoute;

			await tick();
			if (thisNavigation !== navigationId) return;

			const position = scrollPositions.get(toRoute) ?? { x: 0, y: 0 };
			window.scrollTo(position.x, position.y);
		}

		document.addEventListener('click', handleNavigationClick, true);
		window.addEventListener('scroll', saveCurrentScrollPosition);
		window.addEventListener('popstate', saveCurrentScrollPosition);
		window.addEventListener('hashchange', handleHashChange);

		return () => {
			document.removeEventListener('click', handleNavigationClick, true);
			window.removeEventListener('scroll', saveCurrentScrollPosition);
			window.removeEventListener('popstate', saveCurrentScrollPosition);
			window.removeEventListener('hashchange', handleHashChange);
			history.scrollRestoration = previousScrollRestoration;
		};
	});
</script>

<Header />

<main>
	<content hidden={router.location !== '/'} inert={router.location !== '/'}>
		<Home />
	</content>
	<content hidden={router.location !== '/settings'} inert={router.location !== '/settings'}>
		<Settings />
	</content>
	<content hidden={router.location !== '/about'} inert={router.location !== '/about'}>
		<About />
	</content>
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
