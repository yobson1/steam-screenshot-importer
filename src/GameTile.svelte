<script>
	export let imgSrc;
	export let appID;
	export let appName;
	import { onMount } from "svelte";
	import { open } from "@tauri-apps/api/dialog";
	import { pictureDir } from "@tauri-apps/api/path";
	import { importScreenshots } from "./screenshots.js";
	import VanillaTilt from "vanilla-tilt";
	import swal from "sweetalert";

	let tile;
	let img;
	onMount(() => {
		VanillaTilt.init(tile, {
			reverse: true,
			max: 12,
			perspective: 900,
			scale: 1.1,
			axis: "y",
			easing: "cubic-bezier(.03, .7, .8 ,1)",
			glare: true,
			"max-glare": 0.5,
		});

		tile.onmouseover = () => {
			tile.style.margin = "1rem";
			img.style.transform = tile.style.transform;
			tile.oldPad = tile.style.padding;
			tile.style.padding = 0;
		};

		tile.onmouseout = () => {
			tile.style.margin = "0";
			img.style.transform = null;
			tile.style.padding = tile.oldPad;
		};

		tile.onclick = () => {
			pictureDir().then((dir) => {
				// https://github.com/image-rs/image#supported-image-formats
				open({
					defaultPath: dir,
					filters: [
						{
							name: "Images",
							extensions: [
								"png",
								"jpg",
								"jpeg",
								"bmp",
								"ico",
								"tiff",
								"tif",
								"webp",
								"avif",
								"pnm",
								"dds",
								"tga",
								"exr",
							],
						},
					],
					multiple: true,
					title: "Select screenshots to import",
				}).then((files) => {
					importScreenshots(files, appID).then((err) => {
						if (err) {
							swal({
								title: "Error",
								text: err,
								icon: "error",
							});

							console.error(err);
						}

						swal.close();
					});
				});
			});
		};

		img.onerror = () => {
			img.src = "defaultappimage.png";
			let title = tile.querySelector("span");
			title.style.visibility = "visible";
		};
	});
</script>

<div bind:this={tile}>
	<img bind:this={img} src={imgSrc} alt={appName} />
	<span class="no-img-title">{appName}</span>
</div>

<style>
	div {
		position: relative;
		transform-style: preserve-3d;
		text-align: center;
		float: left;
		transition: margin var(--transition-speed);
		user-select: none;
		width: 210px;
		height: 315px;
		padding: 0.8rem;
		cursor: pointer;
	}

	span.no-img-title {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%) translateZ(20px);
		font-size: 1.5em;
		color: #fff;
		text-shadow: 1px 1px 1px rgba(0, 0, 0, 0.3);
		visibility: hidden;
		user-select: none;
		pointer-events: none;
		box-sizing: border-box;
		overflow: hidden;
		color: #b9c2cc;
	}

	div:hover img {
		filter: none;
	}

	img {
		width: 210px;
		height: 315px;
		user-select: none;
		pointer-events: none;
		transition: filter 0.8s;
		-webkit-filter: drop-shadow(0 5px 10px black);
		filter: drop-shadow(0 5px 10px black);
		-webkit-transform: translateZ(0);
		transform: translateZ(0);
	}
</style>
