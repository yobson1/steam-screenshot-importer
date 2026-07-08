<script lang="ts">
	import { screenshotSettings, FILTER_TYPES, type FilterType } from './settings.store.svelte';

	const filterLabels: Record<FilterType, string> = {
		Nearest: 'Nearest Neighbor: fastest, blockiest',
		Triangle: 'Triangle: bilinear',
		CatmullRom: 'Catmull-Rom: bicubic',
		Gaussian: 'Gaussian',
		Lanczos3: 'Lanczos3: best quality, slowest'
	};

	function onQualityInput(event: Event) {
		screenshotSettings.setQuality(Number((event.target as HTMLInputElement).value));
	}

	function onFilterChange(event: Event) {
		screenshotSettings.setFilterType((event.target as HTMLSelectElement).value as FilterType);
	}

	function onCheckUpdatesChange(event: Event) {
		screenshotSettings.setCheckUpdatesOnStartup((event.target as HTMLInputElement).checked);
	}
</script>

<h1>Options</h1>

<form class="settings-form">
	<fieldset>
		<legend>Image Processing</legend>

		<label for="jpeg-quality">
			JPEG quality
			<span class="value-badge">{screenshotSettings.jpegQuality}</span>
		</label>
		<input
			id="jpeg-quality"
			type="range"
			min="1"
			max="100"
			value={screenshotSettings.jpegQuality}
			oninput={onQualityInput}
		/>
		<p class="hint">
			Used when converting non-JPEG images and when generating the Steam thumbnail. Higher is better
			quality but a larger file size.
		</p>

		<label for="filter-type">Downscale filter</label>
		<select id="filter-type" value={screenshotSettings.filterType} onchange={onFilterChange}>
			{#each FILTER_TYPES as filter (filter)}
				<option value={filter}>{filterLabels[filter]}</option>
			{/each}
		</select>
		<p class="hint">
			Algorithm used when an image needs to be resized to fit within Steam's limits, and when
			generating the thumbnail.
		</p>
	</fieldset>

	<fieldset>
		<legend>Updates</legend>

		<label for="check-updates" class="checkbox-label">
			<input
				id="check-updates"
				type="checkbox"
				checked={screenshotSettings.checkUpdatesOnStartup}
				onchange={onCheckUpdatesChange}
			/>
			Check for updates on startup
		</label>
	</fieldset>
</form>

<style>
	.settings-form {
		max-width: 480px;
		margin: 0 auto;
		text-align: left;
		padding-inline: 1rem;
	}

	fieldset {
		border: 1px solid rgba(128, 128, 128, 0.3);
		border-radius: 8px;
		padding: 1.25rem 1.5rem 1.5rem;
		margin-top: 1rem;
	}

	legend {
		padding-inline: 0.5rem;
		font-weight: 600;
	}

	label {
		margin-top: 1rem;
		font-weight: 600;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.value-badge {
		background-color: var(--accent);
		color: white;
		border-radius: 6px;
		padding: 0.1rem 0.5rem;
		font-size: 0.85em;
		font-weight: 700;
	}

	input[type='range'] {
		width: 100%;
		accent-color: var(--accent);
	}

	select {
		width: 100%;
	}

	.checkbox-label {
		display: flex;
		justify-content: flex-start;
		align-items: center;
		gap: 0.5rem;
		font-weight: 600;
	}

	.checkbox-label input[type='checkbox'] {
		width: auto;
		margin: 0;
		accent-color: var(--accent);
	}

	.hint {
		margin-top: 0.25rem;
		font-size: 0.85em;
		opacity: 0.7;
		font-weight: 400;
	}
</style>
