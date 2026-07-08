<script lang="ts">
	import GameTile from './GameTile.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Swal from 'sweetalert2';
	import Fuse from 'fuse.js';

	type Game = [appId: number, base64Img: string, appName: string];

	const SEARCH_DEBOUNCE_MS = 200;

	let get_games_prom = invoke<Game[]>('get_games');
	let steam_user_prom = invoke<string>('get_recent_steam_user');

	let games = $state<Game[] | null>(null);
	let gamesError = $state<string | null>(null);

	get_games_prom
		.then((result) => {
			games = result;
		})
		.catch((error) => {
			gamesError = error;
			Swal.fire('Error', error, 'error');
		});

	let searchQuery = $state('');
	let debouncedQuery = $state('');
	let debounceTimeout: ReturnType<typeof setTimeout>;

	$effect(() => {
		const query = searchQuery;

		clearTimeout(debounceTimeout);
		debounceTimeout = setTimeout(() => {
			debouncedQuery = query;
		}, SEARCH_DEBOUNCE_MS);

		return () => clearTimeout(debounceTimeout);
	});

	let fuse = $derived(
		games ? new Fuse(games, { keys: ['2'], threshold: 0.4, ignoreLocation: true }) : null
	);

	let filteredGames = $derived(
		games === null
			? []
			: debouncedQuery.trim() === ''
				? games
				: (fuse?.search(debouncedQuery).map((result) => result.item) ?? [])
	);

	function getSteamLibraryImage(appID: number) {
		return invoke<string | null>('get_library_image', { appId: appID });
	}
</script>

<content>
	{#await steam_user_prom}
		<h1>Welcome user!</h1>
	{:then steam_username}
		<h1>Welcome {steam_username}!</h1>
	{:catch error}
		<p>Error: {error}</p>
	{/await}

	{#if games === null}
		<p>{gamesError ? `Error: ${gamesError}` : 'Fetching games.'}</p>
	{:else}
		<div class="search-container">
			<div class="search-wrapper">
				<input
					type="search"
					class="game-search"
					placeholder="Search your games…"
					bind:value={searchQuery}
					aria-label="Search games"
				/>
				<!-- si:search-duotone from https://sargamicons.com/ -->
				<svg
					class="search-icon"
					xmlns="http://www.w3.org/2000/svg"
					width="1.2em"
					height="1.2em"
					viewBox="0 0 24 24"
				>
					<path d="M0 0h24v24H0z" fill="none" />
					<g fill="none">
						<path fill="currentColor" fill-opacity=".16" d="M11 19a8 8 0 1 0 0-16a8 8 0 0 0 0 16" />
						<path
							stroke="currentColor"
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-miterlimit="10"
							stroke-width="1.5"
							d="m21 21l-4-4m2-6a8 8 0 1 1-16 0a8 8 0 0 1 16 0"
						/>
					</g>
				</svg>
			</div>
		</div>

		{#if filteredGames.length === 0}
			<p>No games found matching "{debouncedQuery}"</p>
		{:else}
			<section class="tiles">
				{#each filteredGames as game (game[0])}
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
		{/if}
	{/if}
</content>

<style>
	.search-container {
		display: flex;
		justify-content: center;
		margin-bottom: 1.5rem;
	}

	.search-wrapper {
		position: relative;
		width: 100%;
		max-width: 520px;
	}

	.search-icon {
		position: absolute;
		top: 50%;
		left: 0.9rem;
		transform: translateY(-50%);
		color: var(--content-light);
		opacity: 0.5;
		pointer-events: none;
		transition: opacity var(--transition-speed);
	}

	.game-search {
		width: 100%;
		margin: 0;
		padding: 0.7rem 1rem 0.7rem 2.6rem;
		border-radius: 999px;
		border: 1px solid rgba(128, 128, 128, 0.3);
		background-color: rgba(0, 0, 0, 0.03);
		color: var(--content-light);
		font-size: 1rem;
		box-shadow: none;
		transition:
			border-color var(--transition-speed),
			box-shadow var(--transition-speed),
			background-color var(--transition-speed);
	}

	.game-search::placeholder {
		color: var(--content-light);
		opacity: 0.45;
	}

	.game-search:hover {
		border-color: rgba(128, 128, 128, 0.5);
	}

	.game-search:focus {
		outline: none;
		border-color: var(--accent);
		box-shadow: 0 0 0 3px rgba(255, 62, 0, 0.2);
	}

	.game-search:focus ~ .search-icon {
		opacity: 0.9;
		color: var(--accent);
	}

	/* Clear ("x") button that browsers add to type="search" inputs */
	.game-search::-webkit-search-cancel-button {
		filter: none;
		opacity: 0.6;
		cursor: pointer;
	}

	:global(body.dark-mode) .game-search {
		background-color: rgba(255, 255, 255, 0.05);
		border-color: rgba(255, 255, 255, 0.12);
		color: var(--content-dark);
	}

	:global(body.dark-mode) .game-search::placeholder {
		color: var(--content-dark);
		opacity: 0.4;
	}

	:global(body.dark-mode) .game-search:hover {
		border-color: rgba(255, 255, 255, 0.25);
	}

	:global(body.dark-mode) .search-icon {
		color: var(--content-dark);
	}

	section.tiles {
		display: flex;
		flex-wrap: wrap;
		flex-direction: row;
		justify-content: center;
	}
</style>
