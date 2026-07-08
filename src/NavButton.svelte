<script lang="ts">
	import type { MenuButton } from './types';

	let { name, href, src, rotate, onclick = () => {} }: MenuButton = $props();
</script>

<a href={href ? '#' + href : undefined}>
	<button {onclick}>
		<span class="img-container">
			<img class={rotate ? 'rotate' : ''} {src} alt={name} />
		</span>
		<span>{name}</span>
	</button>
</a>

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
		background-color: var(--button-background-color);
		color: var(--button-content-color);
		border-radius: 8px;
		box-shadow: 0 0 3px 2px rgba(0, 0, 0, 0.25);
		transition: filter 200ms ease-in-out;
	}

	img {
		filter: var(--nav-icon-filter);
		transition: transform 750ms ease;
	}

	span {
		display: block;
		filter: none;
	}

	button:hover {
		filter: var(--nav-button-hover-filter);
	}

	button:hover img.rotate {
		transform: rotate(360deg);
	}

	.img-container {
		transition: transform 100ms ease;
	}

	button:active .img-container {
		transform: scale(0.85);
	}
</style>
