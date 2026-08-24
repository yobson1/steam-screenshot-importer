<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { importScreenshots } from './screenshots';
	import VanillaTilt from 'vanilla-tilt';

	let { imgSrc, appID, appName }: { imgSrc?: string; appID: number; appName: string } = $props();

	let tile: HTMLDivElement & { vanillaTilt?: VanillaTilt };
	let imageFailed = $state(false);

	function handleImgErr() {
		imageFailed = true;
	}

	function handleActivate() {
		importScreenshots(appID);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			handleActivate();
		}
	}

	onMount(() => {
		VanillaTilt.init(tile, {
			reverse: true,
			max: 12,
			perspective: 900,
			scale: 1.1,
			axis: 'y',
			easing: 'cubic-bezier(.03, .7, .8 ,1)',
			glare: true,
			'max-glare': 0.5
		});
	});

	onDestroy(() => {
		tile.vanillaTilt?.destroy();
	});
</script>

<div
	bind:this={tile}
	role="button"
	tabindex="0"
	aria-label="Import screenshots for {appName}"
	onclick={handleActivate}
	onkeydown={handleKeydown}
>
	{#if imageFailed}
		<img src="defaultappimage.png" alt={appName} />
		<span class="no-img-title">{appName}</span>
	{:else}
		<img src={imgSrc} alt={appName} onerror={handleImgErr} />
	{/if}
</div>

<style>
	div {
		position: relative;
		transform-style: preserve-3d;
		text-align: center;
		float: left;
		user-select: none;
		width: 212px;
		height: 318px;
		cursor: pointer;
		box-sizing: border-box;
		margin: 1rem;
	}

	span.no-img-title {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%) translateZ(20px);
		font-size: 1.5em;
		color: #fff;
		text-shadow: 1px 1px 1px rgba(0, 0, 0, 0.3);
		user-select: none;
		pointer-events: none;
		overflow: hidden;
		color: #b9c2cc;
	}

	div:hover img {
		filter: none;
	}

	img {
		width: 100%;
		height: 100%;
		user-select: none;
		pointer-events: none;
		transition: filter 0.8s;
		-webkit-filter: drop-shadow(0 5px 10px black);
		filter: drop-shadow(0 5px 10px black);
		-webkit-transform: translateZ(0);
		transform: translateZ(0);
	}
</style>
