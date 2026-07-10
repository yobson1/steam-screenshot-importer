import type { ResizeFilterType } from './bindings';

export type FilterType = ResizeFilterType;

export const FILTER_LABELS = {
	Nearest: 'Nearest Neighbor: fastest, blockiest',
	Triangle: 'Triangle: bilinear',
	CatmullRom: 'Catmull-Rom: bicubic',
	Gaussian: 'Gaussian',
	Lanczos3: 'Lanczos3: best quality, slowest'
} satisfies Record<ResizeFilterType, string>;

export const FILTER_TYPES = Object.keys(FILTER_LABELS) as ResizeFilterType[];

const DEFAULT_QUALITY = 95;
const DEFAULT_FILTER: FilterType = 'Lanczos3';
const DEFAULT_CHECK_UPDATES_ON_STARTUP = true;

class ScreenshotSettings {
	jpegQuality = $state(loadQuality());
	filterType = $state(loadFilterType());
	checkUpdatesOnStartup = $state(loadCheckUpdatesOnStartup());

	setQuality(value: number) {
		const clamped = Math.min(100, Math.max(1, Math.round(value)));
		this.jpegQuality = clamped;
		localStorage.setItem('jpegQuality', clamped.toString());
	}

	setFilterType(value: FilterType) {
		this.filterType = value;
		localStorage.setItem('filterType', value);
	}

	setCheckUpdatesOnStartup(value: boolean) {
		this.checkUpdatesOnStartup = value;
		localStorage.setItem('checkUpdatesOnStartup', value.toString());
	}
}

function loadQuality(): number {
	const stored = localStorage.getItem('jpegQuality');
	const parsed = stored ? parseInt(stored) : NaN;
	return !isNaN(parsed) && parsed >= 1 && parsed <= 100 ? parsed : DEFAULT_QUALITY;
}

function loadFilterType(): FilterType {
	const stored = localStorage.getItem('filterType');
	return FILTER_TYPES.includes(stored as FilterType) ? (stored as FilterType) : DEFAULT_FILTER;
}

function loadCheckUpdatesOnStartup(): boolean {
	const stored = localStorage.getItem('checkUpdatesOnStartup');
	return stored === null ? DEFAULT_CHECK_UPDATES_ON_STARTUP : stored === 'true';
}

export const screenshotSettings = new ScreenshotSettings();
