<script lang="ts">
	import GameTile from './GameTile.svelte';
	import { commands, type Game } from './bindings';
	import Swal from 'sweetalert2';
	import Fuse from 'fuse.js';

	const SEARCH_DEBOUNCE_MS = 200;

	let gamesPromise = commands.getGames();
	let steamUserPromise = commands.getRecentSteamUser();

	let games = $state<Game[] | null>(null);
	let gamesError = $state<string | null>(null);

	gamesPromise
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
		games ? new Fuse(games, { keys: ['appName'], threshold: 0.4, ignoreLocation: true }) : null
	);

	let filteredGames = $derived(
		games === null
			? []
			: debouncedQuery.trim() === ''
				? games
				: (fuse?.search(debouncedQuery).map((result) => result.item) ?? [])
	);
</script>

<content>
	{#await steamUserPromise}
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
				{#each filteredGames as game (game.appId)}
					<GameTile appID={game.appId} appName={game.appName} imgSrc={game.imageSrc} />
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
		color: var(--content-color);
		opacity: 0.5;
		pointer-events: none;
		transition: opacity var(--transition-speed);
	}

	.game-search {
		width: 100%;
		margin: 0;
		padding-inline-start: 2.6rem;
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

	section.tiles {
		display: flex;
		flex-wrap: wrap;
		flex-direction: row;
		justify-content: center;
	}
</style>
