<script>
	import { onMount } from "svelte";
	import confetti from "canvas-confetti";

	let btn;

	onMount(() => {
		btn.onclick = (event) => {
			confetti({
				origin: {
					x: event.clientX / window.visualViewport.width,
					y: event.clientY / window.visualViewport.height,
				},
				angle: 130,
				ticks: 300,
				particleCount: 100,
				disableForReducedMotion: true,
			});
		};
	});

	export let href,
		src,
		alt,
		size = null;
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
