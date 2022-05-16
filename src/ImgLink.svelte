<script>
	let btn;

	// TODO: Use pixi.js
	// https://www.npmjs.com/package/pixi.js
	// https://www.npmjs.com/package/@types/pixi.js
	// To make a floating heart animation when the button is clicked

	export let href, src, alt;
</script>

<a draggable="false" {href} target="_blank" bind:this={btn}
	><img {src} {alt} /></a
>

<style>
	a,
	img {
		user-select: none;
	}

	img {
		pointer-events: none;
	}
</style>
