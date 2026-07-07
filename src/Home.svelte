<script lang="ts">
	import GameTile from './GameTile.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Swal from 'sweetalert2';

	let get_games_prom = invoke<[number, string, string][]>('get_games');
	let steam_user_prom = invoke<string>('get_recent_steam_user');

	get_games_prom.catch((error) => {
		Swal.fire('Error', error, 'error');
	});

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

	{#await get_games_prom}
		<p>Fetching games.</p>
	{:then games}
		<section class="tiles">
			{#each games as game (game[0])}
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

<style>
	section.tiles {
		display: flex;
		flex-wrap: wrap;
		flex-direction: row;
		justify-content: center;
	}
</style>
