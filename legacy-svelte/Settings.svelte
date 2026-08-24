<script lang="ts">
	import {
		screenshotSettings,
		FILTER_LABELS,
		FILTER_TYPES,
		type FilterType
	} from './settings.store.svelte';
	import runUpdateCheck from './updater';

	let checkingForUpdates = $state(false);

	function onQualityInput(event: Event) {
		screenshotSettings.setQuality(Number((event.target as HTMLInputElement).value));
	}

	function onFilterChange(event: Event) {
		screenshotSettings.setFilterType((event.target as HTMLSelectElement).value as FilterType);
	}

	function onCheckUpdatesChange(event: Event) {
		screenshotSettings.setCheckUpdatesOnStartup((event.target as HTMLInputElement).checked);
	}

	async function handleCheckForUpdates() {
		checkingForUpdates = true;
		try {
			await runUpdateCheck(true);
		} finally {
			checkingForUpdates = false;
		}
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
				<option value={filter}>{FILTER_LABELS[filter]}</option>
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

		<button
			type="button"
			class="btn-accent check-updates-btn"
			onclick={handleCheckForUpdates}
			disabled={checkingForUpdates}
		>
			{checkingForUpdates ? 'Checking…' : 'Check for updates now'}
		</button>
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
		padding: 0.7rem 1rem;
		border: 1px solid var(--text-input-border-color);
		border-radius: 8px;
		background-color: var(--text-input-background-color);
		color: var(--content-color);
		box-shadow: none;
		cursor: pointer;
		transition:
			border-color var(--transition-speed),
			box-shadow var(--transition-speed),
			background-color var(--transition-speed);
	}

	select:hover {
		border-color: var(--text-input-hover-border-color);
	}

	select:focus {
		outline: none;
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--text-input-focus-ring-color);
	}

	option {
		background-color: var(--background-color);
		color: var(--content-color);
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

	.check-updates-btn {
		margin-top: 1rem;
		width: 100%;
	}

	.hint {
		margin-top: 0.25rem;
		font-size: 0.85em;
		opacity: 0.7;
		font-weight: 400;
	}
</style>
