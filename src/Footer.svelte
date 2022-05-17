<script lang="ts">
	import ImgLink from "./ImgLink.svelte";
	import { darkModeEnabled } from "./stores.js";
	import { getVersion } from "@tauri-apps/api/app";
	import { onMount } from "svelte";

	let versionProm = getVersion();

	let version;
	let augh = new Audio("/easter/juiced.mp3");
	augh.volume = 0.25;
	onMount(() => {
		// Trigger an easter egg if the version number is clicked 6 times
		version.onclick = (event: MouseEvent) => {
			if (event.detail % 6 === 0) {
				// Create image in the center of the screen
				let img = document.createElement("img");
				img.src = "/easter/juiced.webp";
				img.style.position = "absolute";
				img.style.top = "50%";
				img.style.left = "50%";
				img.style.transform = "translate(-50%, -50%)";
				img.style.width = "100%";
				img.style.height = "100%";
				img.style.zIndex = "5";
				document.body.appendChild(img);

				// Disable scrolling
				let oldOverflow = document.body.style.overflow;
				document.body.style.overflow = "hidden";

				// Play sound
				augh.loop = true;
				augh.play();

				// Let it be disabled with a click after 3s
				setTimeout(() => {
					img.onclick = () => {
						document.body.style.overflow = oldOverflow;
						img.remove();
						augh.pause();
						augh.currentTime = 0;
					};
				}, 3000);
			}
		};
	});
</script>

<footer>
	<nav class="left">
		<p class="version" bind:this={version}>
			{#await versionProm then version}
				v{version}
			{/await}
		</p>
	</nav>
	<nav class="right">
		<ImgLink
			href="https://buymeacoffee.com/yobson"
			src="bmc-logo.svg"
			alt="buymeacoffee"
			size="32"
		/>

		<ImgLink
			href="https://github.com/yobson1"
			src={$darkModeEnabled
				? "GitHub-Mark-Light-32px.png"
				: "GitHub-Mark-32px.png"}
			alt="GitHub"
		/>
	</nav>
</footer>

<style>
	.version {
		margin-inline-start: 1em;
		cursor: default;
		user-select: none;
	}

	footer {
		display: flex;
		position: fixed;
		bottom: 0;
		z-index: 1;
		width: calc(100% - (var(--body-padding) * 2));
	}

	nav {
		margin: 0;
		margin-bottom: 0.5rem;
		padding: 0;
		width: 50%;
		display: flex;
	}

	nav.left {
		justify-content: flex-start;
	}

	nav.right {
		justify-content: flex-end;
	}
</style>
