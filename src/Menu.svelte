<script lang="ts">
	import { fly } from 'svelte/transition';
	import NavButton from './NavButton.svelte';
	import Swal from 'sweetalert2';
	import { importScreenshots } from './screenshots';
	import { onMount } from 'svelte';
	import type { MenuButton } from './types';

	let { open = $bindable(false), width = 96 }: { open?: boolean; width?: number } = $props();

	let menu: HTMLDivElement;
	onMount(() => {
		document.body.onclick = (event) => {
			const path = event.composedPath();
			if (
				!path.includes(menu) &&
				!path.includes(document.getElementsByClassName('hamburger')[0]) &&
				!path.includes(document.getElementsByClassName('swal2-container')[0])
			) {
				open = false;
			}
		};
	});

	let buttons: MenuButton[] = [
		{ name: 'Home', href: '/', src: 'home.svg', rotate: false },
		{
			name: 'App ID',
			src: 'plus-circle.svg',
			rotate: true,
			onclick: async () => {
				const result = await Swal.fire({
					title: 'Custom App ID',
					input: 'text',
					inputPlaceholder: 'Enter custom app ID',
					showCancelButton: true,
					confirmButtonText: 'Import',
					cancelButtonText: 'Cancel'
				});

				if (result.isConfirmed && result.value != null) {
					const appIDInt = parseInt(result.value);
					if (isNaN(appIDInt)) {
						await Swal.fire({
							title: 'Invalid App ID',
							text: 'Please enter a valid app ID',
							icon: 'error'
						});
					} else {
						importScreenshots(appIDInt);
					}
				}
			}
		},
		{ name: 'About', href: '/about', src: 'info.svg', rotate: true },
		{ name: 'Options', href: '/settings', src: 'settings.svg', rotate: true }
	];
</script>

<div
	bind:this={menu}
	class="menu"
	style="left: {open ? 'calc(-1 * var(--body-padding))' : 'calc(-' + width + 'px - 2.5rem)'};
		width: {width}px;
		box-shadow: {open ? '0 0 4px 5px rgba(0, 0, 0, 0.4)' : 'none'};"
>
	{#if open}
		{#each buttons as button (button.name)}
			<p transition:fly={{ duration: 525 }}>
				<NavButton {...button} />
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
		background-color: var(--menu-background-color);
		color: var(--content-color);
		transition: none 525ms ease-in-out;
		transition-property: box-shadow, left;
		overflow: auto;
	}

	p {
		margin: 1rem auto;
	}
</style>
