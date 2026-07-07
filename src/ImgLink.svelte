<script lang="ts">
	import { onMount } from 'svelte';
	import confetti from 'canvas-confetti';
	import { openUrl } from '@tauri-apps/plugin-opener';

	let btn = $state();

	onMount(() => {
		btn.onclick = (event) => {
			event.preventDefault();

			confetti({
				origin: {
					x: event.clientX / window.visualViewport.width,
					y: event.clientY / window.visualViewport.height
				},
				angle: 130,
				ticks: 300,
				particleCount: 100,
				disableForReducedMotion: true
			});

			openUrl(href);
		};
	});

	interface Props {
		href: any;
		src: any;
		alt: any;
		size?: any;
	}

	let {
		href,
		src,
		alt,
		size = null
	}: Props = $props();
</script>

<a draggable="false" {href} target="_blank" bind:this={btn}>
	<img {src} {alt} width={size} height={size} />
</a>

<style>
	a,
	img {
		user-select: none;
	}

	img {
		pointer-events: none;
		padding-inline: 0.5rem;
	}
</style>
