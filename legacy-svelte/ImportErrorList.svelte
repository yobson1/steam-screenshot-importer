<script lang="ts">
	import type { ImportFailure } from './bindings';

	let { errors }: { errors: ImportFailure[] } = $props();

	function fileName(filePath: string): string {
		return filePath.split(/[\\/]/).pop() || filePath;
	}
</script>

<ul>
	{#each errors as error, index (`${error.filePath}-${index}`)}
		<li>
			<strong>{fileName(error.filePath)}</strong>
			<span>{error.filePath}</span>
			<p>{error.message}</p>
		</li>
	{/each}
</ul>

<style>
	ul {
		display: grid;
		gap: 0.75rem;
		margin: 0;
		padding: 0;
		list-style: none;
		text-align: left;
	}

	li {
		padding: 0.8rem 1rem;
		border-left: 3px solid var(--danger);
		border-radius: 4px;
		background-color: var(--text-input-background-color);
	}

	strong,
	span {
		display: block;
		overflow-wrap: anywhere;
	}

	strong {
		font-size: 1rem;
	}

	span {
		margin-top: 0.15rem;
		font-size: 0.75rem;
		opacity: 0.65;
	}

	p {
		margin: 0.65rem 0 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
</style>
