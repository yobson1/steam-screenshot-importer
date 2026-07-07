<script lang="ts">
	import Header from './Header.svelte';
	import Footer from './Footer.svelte';
	import GameTile from './GameTile.svelte';
	import Settings from './Settings.svelte';
	import About from './About.svelte';
	import { Route as TinroRoute } from 'tinro';
	import type { Component, Snippet } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Swal from 'sweetalert2';

	const Route = TinroRoute as unknown as Component<{ path?: string; children?: Snippet }>;

	let get_games_prom = invoke<[number, string, string][]>('get_games');
	let steam_user_prom = invoke<string>('get_recent_steam_user');

	get_games_prom.catch((error) => {
		Swal.fire('Error', error, 'error');
	});

	function getSteamLibraryImage(appID: number) {
		return invoke<string | null>('get_library_image', { appId: appID });
	}
</script>

<Header />

<Route path="/">
	<main>
		<content>
			{#await steam_user_prom}
				<h1>Welcome user!</h1>
			{:then steam_username}
				<h1>Welcome {steam_username}!</h1>
			{:catch error}
				<p>Error: {error}</p>
			{/await}

			{#await get_games_prom}
				<p>Fetching games.</p>
			{:then games}
				<section class="tiles">
					{#each games as game}
						{#if game[1] !== ''}
							<GameTile
								appID={game[0]}
								appName={game[2]}
								imgSrc={'data:image/jpeg;base64,' + game[1]}
							/>
						{:else}
							{#await getSteamLibraryImage(game[0])}
								<GameTile appID={game[0]} appName={game[2]} />
							{:then image}
								<GameTile appID={game[0]} appName={game[2]} imgSrc={image ?? ''} />
							{/await}
						{/if}
					{/each}
				</section>
			{:catch error}
				<p>Error: {error}</p>
			{/await}
		</content>
	</main>
</Route>

<Route path="/settings">
	<main>
		<content>
			<Settings />
		</content>
	</main>
</Route>

<Route path="/about">
	<main>
		<content>
			<About />
		</content>
	</main>
</Route>

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

	content {
		text-align: center;
	}

	section.tiles {
		display: flex;
		flex-wrap: wrap;
		flex-direction: row;
		justify-content: center;
	}

	@media (min-width: 640px) {
		main {
			max-width: none;
		}
	}
</style>
