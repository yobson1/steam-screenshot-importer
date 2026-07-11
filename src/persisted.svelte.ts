type Validator<T> = (raw: string) => T | undefined;

function load<T>(key: string, defaultValue: T, validate: Validator<T>): T {
	const stored = localStorage.getItem(key);
	if (stored === null) return defaultValue;

	const parsed = validate(stored);
	return parsed === undefined ? defaultValue : parsed;
}

/**
 * A $state value that's automatically loaded from and persisted to
 * localStorage, with validation on read so corrupt/stale storage
 * falls back to a default instead of producing bad state.
 */
export class Persisted<T extends string | number | boolean> {
	#key: string;
	#value = $state<T>() as T;

	constructor(key: string, defaultValue: T, validate: Validator<T>) {
		this.#key = key;
		this.#value = load(key, defaultValue, validate);
	}

	get value(): T {
		return this.#value;
	}

	set(value: T) {
		this.#value = value;
		localStorage.setItem(this.#key, String(value));
	}
}

// Common validators
export const asBoolean: Validator<boolean> = (raw) =>
	raw === 'true' ? true : raw === 'false' ? false : undefined;

export const asIntInRange =
	(min: number, max: number): Validator<number> =>
	(raw) => {
		const parsed = parseInt(raw);
		return !isNaN(parsed) && parsed >= min && parsed <= max ? parsed : undefined;
	};

export const asEnum =
	<T extends string>(values: readonly T[]): Validator<T> =>
	(raw) =>
		values.includes(raw as T) ? (raw as T) : undefined;
