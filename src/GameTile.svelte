<script>
	export let imgSrc;
	export let appID;
	export let appName;
	import { onMount } from 'svelte';
	import { importScreenshots } from './screenshots.js';
	import VanillaTilt from 'vanilla-tilt';

	let tile;
	let img;
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

		tile.onclick = () => {
			importScreenshots(appID);
		};

		img.onerror = () => {
			img.src = 'defaultappimage.png';
			let title = tile.querySelector('span');
			title.style.visibility = 'visible';
		};
	});
</script>

<div bind:this={tile}>
	<img bind:this={img} src={imgSrc} alt={appName} />
	<span class="no-img-title">{appName}</span>
</div>

<style>
	div {
		position: relative;
		transform-style: preserve-3d;
		text-align: center;
		float: left;
		user-select: none;
		width: 210px;
		height: 315px;
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
		visibility: hidden;
		user-select: none;
		pointer-events: none;
		overflow: hidden;
		color: #b9c2cc;
	}

	div:hover img {
		filter: none;
	}

	img {
		width: 210px;
		height: 315px;
		user-select: none;
		pointer-events: none;
		transition: filter 0.8s;
		-webkit-filter: drop-shadow(0 5px 10px black);
		filter: drop-shadow(0 5px 10px black);
		-webkit-transform: translateZ(0);
		transform: translateZ(0);
	}
</style>
