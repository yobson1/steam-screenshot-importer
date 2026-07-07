export type FilterType = 'Nearest' | 'Triangle' | 'CatmullRom' | 'Gaussian' | 'Lanczos3';

export const FILTER_TYPES: FilterType[] = [
	'Nearest',
	'Triangle',
	'CatmullRom',
	'Gaussian',
	'Lanczos3'
];

const DEFAULT_QUALITY = 95;
const DEFAULT_FILTER: FilterType = 'Lanczos3';

class ScreenshotSettings {
	jpegQuality = $state(loadQuality());
	filterType = $state(loadFilterType());

	setQuality(value: number) {
		const clamped = Math.min(100, Math.max(1, Math.round(value)));
		this.jpegQuality = clamped;
		localStorage.setItem('jpegQuality', clamped.toString());
	}

	setFilterType(value: FilterType) {
		this.filterType = value;
		localStorage.setItem('filterType', value);
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

export const screenshotSettings = new ScreenshotSettings();
