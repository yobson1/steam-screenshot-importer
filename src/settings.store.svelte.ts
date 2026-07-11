import type { ResizeFilterType } from './bindings';
import { Persisted, asBoolean, asIntInRange, asEnum } from './persisted.svelte';

export type FilterType = ResizeFilterType;

export const FILTER_LABELS = {
	Nearest: 'Nearest Neighbor: fastest, blockiest',
	Triangle: 'Triangle: bilinear',
	CatmullRom: 'Catmull-Rom: bicubic',
	Gaussian: 'Gaussian',
	Lanczos3: 'Lanczos3: best quality, slowest'
} satisfies Record<ResizeFilterType, string>;

export const FILTER_TYPES = Object.keys(FILTER_LABELS) as ResizeFilterType[];

class ScreenshotSettings {
	#quality = new Persisted('jpegQuality', 95, asIntInRange(1, 100));
	#filterType = new Persisted<FilterType>('filterType', 'Lanczos3', asEnum(FILTER_TYPES));
	#checkUpdatesOnStartup = new Persisted('checkUpdatesOnStartup', true, asBoolean);

	get jpegQuality() {
		return this.#quality.value;
	}
	get filterType() {
		return this.#filterType.value;
	}
	get checkUpdatesOnStartup() {
		return this.#checkUpdatesOnStartup.value;
	}

	setQuality(value: number) {
		this.#quality.set(Math.min(100, Math.max(1, Math.round(value))));
	}

	setFilterType(value: FilterType) {
		this.#filterType.set(value);
	}

	setCheckUpdatesOnStartup(value: boolean) {
		this.#checkUpdatesOnStartup.set(value);
	}
}

export const screenshotSettings = new ScreenshotSettings();
