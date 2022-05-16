<script>
	import ImgLink from "./ImgLink.svelte";
	import { darkModeEnabled } from "./stores.js";
</script>

<footer>
	<nav class="left" />
	<nav class="right">
		<!-- TODO: Add light mode support -->

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
