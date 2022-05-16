<script>
	export let name, href, src;
</script>

<a {href}><button><img {src} alt={name} /><span>{name}</span></button></a>

<style>
	button {
		cursor: pointer;
		border: none;
		padding: auto;
		text-align: center;
		display: inline-block;
		font-size: 14px;
		font-weight: 600;
		width: 64px;
		height: 64px;
		background-color: white;
		color: black;
		border-radius: 8px;
		box-shadow: 0 0 3px 2px rgba(0, 0, 0, 0.25);
		transition: filter 200ms ease-in-out;
	}

	img {
		transition: transform 750ms ease;
	}

	span {
		display: block;
		filter: none;
	}

	button:hover {
		filter: brightness(0.8);
	}

	button:hover img {
		transform: rotate(360deg);
	}

	:global(body.dark-mode) img {
		filter: invert();
	}

	:global(body.dark-mode) button {
		background-color: rgb(41, 41, 41);
		color: white;
	}

	:global(body.dark-mode) button:hover {
		filter: brightness(1.2);
	}
</style>
