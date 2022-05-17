<script lang="ts">
	import { fly } from "svelte/transition";
	import NavButton from "./NavButton.svelte";
	import swal from "sweetalert";
	import { importScreenshots } from "./screenshots.js";

	export let open = false;
	export let width = 96;

	// https://www.npmjs.com/package/svelte-simple-modal
	let buttons = [
		{
			name: "Home",
			href: "/",
			src: "home.svg",
			rotate: false,
		},
		{
			name: "App ID",
			href: "/",
			src: "plus-circle.svg",
			rotate: true,
			onclick: () => {
				swal({
					title: "Custom App ID",
					content: {
						element: "input",
						attributes: {
							placeholder: "Enter custom app ID",
						},
					},
				}).then((appID: string) => {
					let appIDInt = parseInt(appID);

					if (isNaN(appIDInt)) {
						swal({
							title: "Invalid App ID",
							text: "Please enter a valid app ID",
							icon: "error",
						});
					} else {
						importScreenshots(appIDInt);
					}
				});
			},
		},
		{
			name: "About",
			href: "/about",
			src: "info.svg",
			rotate: true,
		},
		{
			// TODO: Route to the options page
			name: "Options",
			href: "/settings",
			src: "settings.svg",
			rotate: true,
		},
	];
</script>

<div
	class="menu"
	style="left: {open
		? 'calc(-1 * var(--body-padding))'
		: 'calc(-' + width + 'px - 2.5rem)'};
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
					rotate={button.rotate}
					onclick={button.onclick}
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
		background-color: #eee;
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
