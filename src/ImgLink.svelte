<script lang="ts">
	import confetti from 'canvas-confetti';
	import { openUrl } from '@tauri-apps/plugin-opener';

	let {
		href,
		src,
		alt,
		size = null
	}: { href: string; src: string; alt: string; size?: string | number | null } = $props();

	function handleClick(event: MouseEvent) {
		event.preventDefault();
		confetti({
			origin: {
				x: event.clientX / window.visualViewport!.width,
				y: event.clientY / window.visualViewport!.height
			},
			angle: 130,
			ticks: 300,
			particleCount: 100,
			disableForReducedMotion: true
		});
		openUrl(href);
	}
</script>

<a draggable="false" {href} target="_blank" onclick={handleClick}>
	<img {src} {alt} width={size} height={size} />
</a>

<style>
	a,
	img {
		user-select: none;
	}

	img {
		pointer-events: none;
		padding-inline: 0.5rem;
	}
</style>
