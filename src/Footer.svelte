<script lang="ts">
	import ImgLink from './ImgLink.svelte';
	import { darkModeEnabled } from './stores.svelte';
	import { getVersion } from '@tauri-apps/api/app';

	let versionProm = getVersion();

	let version: HTMLParagraphElement;
	let augh = new Audio('/easter/juiced.mp3');
	augh.volume = 0.25;
	let clickCount = 0;
	let clickTimeout: ReturnType<typeof setTimeout>;

	function handleVersionClick() {
		clickCount++;

		clearTimeout(clickTimeout);

		// Reset after 1 second without clicking
		clickTimeout = setTimeout(() => {
			clickCount = 0;
		}, 1000);

		// Trigger an easter egg if the version number is clicked 6 times
		if (clickCount === 6) {
			clickCount = 0;
			clearTimeout(clickTimeout);

			let img = document.createElement('img');
			img.src = '/easter/juiced.webp';
			img.style.position = 'absolute';
			img.style.top = '50%';
			img.style.left = '50%';
			img.style.transform = 'translate(-50%, -50%)';
			img.style.width = '100%';
			img.style.height = '100%';
			img.style.zIndex = '5';
			document.body.appendChild(img);

			// Disable scrolling
			let oldOverflow = document.body.style.overflow;
			document.body.style.overflow = 'hidden';

			// Scroll to the top
			window.scrollTo(0, 0);

			// Play sound
			augh.loop = true;
			augh.currentTime = 0;

			augh.play().catch((err) => {
				console.error('Audio playback failed:', err);
			});

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
	}
</script>

<footer>
	<nav class="left">
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<p class="version" bind:this={version} onclick={handleVersionClick}>
			{#await versionProm then appVersion}
				v{appVersion}
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
			src={darkModeEnabled.value ? 'GitHub-Mark-Light-32px.png' : 'GitHub-Mark-32px.png'}
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
		pointer-events: none;
	}

	nav > :global(*) {
		pointer-events: auto;
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
