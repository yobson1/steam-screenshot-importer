<script>
	import { fly } from "svelte/transition";
	import NavButton from "./NavButton.svelte";

	export let open = true;
	export let width = 96;

	let buttons = [
		{
			// TODO: Have a text entry pop up to enter a custom appid to upload for
			name: "App ID",
			href: "/",
			src: "plus-circle.svg",
		},
		{
			// TODO: Have a modal popup with about info
			name: "About",
			href: "/",
			src: "info.svg",
		},
		{
			// TODO: Route to the options page
			name: "Options",
			href: "/",
			src: "settings.svg",
		},
	];
</script>

<div
	class="menu"
	style="left: {open
		? 'calc(-1 * var(--body-padding))'
		: 'calc(-' + width + 'px - 2rem)'};
		width: {width}px;
		box-shadow: {open ? '0 0 4px 5px rgba(0, 0, 0, 0.4)' : 'none'};"
>
	{#if open}
		{#each buttons as button}
			<p transition:fly={{ duration: 525 }}>
				<NavButton
					name={button.name}
					src={button.src}
					href={button.href}
				/>
			</p>
		{/each}
	{/if}
</div>

<style>
	.menu {
		top: calc(-1 * var(--body-padding));
		padding: 1rem;
		padding-top: 5rem;
		text-align: center;
		font-size: 1.5rem;
		letter-spacing: 0.15rem;
		height: 100vh;
		position: absolute;
		background-color: rgb(255, 255, 255);
		color: var(--content-light);
		transition: none 525ms ease-in-out;
		transition-property: box-shadow, left;
		overflow: auto;
	}

	:global(body.dark-mode) .menu {
		background-color: rgba(33, 33, 33, 1);
		color: var(--content-dark);
	}

	p {
		margin: 1rem auto;
	}
</style>
