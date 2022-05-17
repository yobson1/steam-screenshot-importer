<script>
	import ImgLink from "./ImgLink.svelte";
	import { darkModeEnabled } from "./stores.js";
	import { getVersion } from "@tauri-apps/api/app";

	let versionProm = getVersion();
</script>

<footer>
	<nav class="left">
		{#await versionProm then version}
			<p class="version">v{version}</p>
		{/await}
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
		pointer-events: none;
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
