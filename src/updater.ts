import swal from 'sweetalert';
import { marked } from 'marked';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';

interface Manifest {
	version: string;
	body: string;
	url: string;
}

interface UpdateResult {
	shouldUpdate: boolean;
	manifest: Manifest;
}

function compareVersions(a: string, b: string): number {
	const pa = a.replace(/^v/, '').split('.').map(Number);
	const pb = b.replace(/^v/, '').split('.').map(Number);

	const len = Math.max(pa.length, pb.length);

	for (let i = 0; i < len; i++) {
		const na = pa[i] ?? 0;
		const nb = pb[i] ?? 0;

		if (na > nb) return 1;
		if (na < nb) return -1;
	}

	return 0;
}

export async function check(): Promise<UpdateResult> {
	const currentVersion = await getVersion();

	const response = await fetch(
		'https://api.github.com/repos/yobson1/steam-screenshot-importer/releases/latest'
	);

	if (!response.ok) {
		throw new Error(`Failed to fetch latest release: ${response.status}`);
	}

	const release = await response.json();

	const latestVersion = release.tag_name.replace(/^v/, '');

	return {
		shouldUpdate: compareVersions(latestVersion, currentVersion) > 0,
		manifest: {
			version: latestVersion,
			body: release.body ?? '',
			url: release.html_url
		}
	};
}

async function runUpdateCheck() {
	try {
		const { shouldUpdate, manifest } = await check();

		if (!shouldUpdate) return;

		const div = document.createElement('div');
		div.className = 'release-notes';
		div.innerHTML = await marked.parse(manifest.body);

		// so it doesn't take over our webview from clicking any links in the markdown
		div.addEventListener('click', async (e) => {
			const target = e.target as HTMLElement;

			const link = target.closest('a');
			if (!link) return;

			e.preventDefault();
			e.stopPropagation();

			const href = link.getAttribute('href');
			if (href) {
				await openUrl(href);
			}
		});

		const open = await swal({
			title: 'Update available',
			content: { element: div },
			icon: 'info',
			buttons: ['Dismiss', 'Open'],
			closeOnClickOutside: true
		});

		if (open) {
			await openUrl(manifest.url);
		}
	} catch (err) {
		console.error('Failed to check for updates:', err);
	}
}

export default runUpdateCheck;
