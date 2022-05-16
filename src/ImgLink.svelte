<script>
	let btn;

	// TODO: Use pixi.js
	// https://www.npmjs.com/package/pixi.js
	// https://www.npmjs.com/package/@types/pixi.js
	// To make a floating heart animation when the button is clicked

	export let href, src, alt, size;
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
