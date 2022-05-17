<h1>About</h1>
<div>
	<p>
		A Tauri app written with Rust & Svelte using the Steamworks API to
		import any image as a screenshot for any game you own.
	</p>
	<p>
		Please report bugs and submit feature requests on the
		<a
			href="https://github.com/yobson1/steam-screenshot-importer/issues"
			target="_blank"
		>
			GitHub repository.
		</a>
	</p>
</div>
<p class="footer">
	by yobson with
	<svg
		class="heart"
		height="32"
		preserveAspectRatio="xMidYMid meet"
		xmlns="http://www.w3.org/2000/svg"
		viewBox="0 0 48 48"
		><path
			fill="#f44336"
			d="M3.605,12h40.79C42.32,8.413,38.443,6,34,6c-4.176,0-7.852,2.137-10,5.371 C21.852,8.137,18.176,6,14,6C9.557,6,5.68,8.413,3.605,12z"
		/><path
			fill="#ff9800"
			d="M2.042,17h43.917c-0.149-1.811-0.7-3.507-1.563-5H3.605C2.742,13.493,2.191,15.189,2.042,17z"
		/><path
			fill="#ffc107"
			d="M2.042,17C2.015,17.33,2,17.663,2,18c0,1.338,0.277,2.677,0.767,4h42.477 C45.727,20.679,46,19.34,46,18c0-0.337-0.015-0.67-0.042-1H2.042z"
		/><path
			fill="#4caf50"
			d="M5.595,27h36.843c1.202-1.614,2.18-3.292,2.805-5H2.767C3.398,23.705,4.385,25.384,5.595,27z"
		/><path
			fill="#2196f3"
			d="M5.595,27c1.315,1.756,2.893,3.438,4.565,5h27.727c1.669-1.56,3.242-3.242,4.551-5H5.595z"
		/><path
			fill="#3f51b5"
			d="M16.268,37h15.505c1.982-1.44,4.116-3.132,6.114-5H10.161C12.159,33.866,14.291,35.559,16.268,37z"
		/><path
			fill="#9c27b0"
			d="M16.268,37C20.487,40.074,24,42,24,42s3.536-1.922,7.773-5H16.268z"
		/></svg
	>
</p>

<style>
	.footer {
		position: absolute;
		bottom: 0.5rem;
		left: 50vw;
		transform: translateX(-50%);
	}

	.heart {
		transform: translateY(calc(1 / 3 * 100%));
	}

	div {
		padding-inline: 3rem;
	}
</style>
