/** Maps source offsets onto one-based line numbers for a single file. */
export class LineIndex {
	readonly #starts: readonly number[];

	private constructor(starts: number[]) {
		this.#starts = Object.freeze(starts);

		Object.freeze(this);
	}

	/**
	 * Index the line starts of a source text once.
	 *
	 * @param text - The complete source text.
	 * @returns An index answering line lookups in logarithmic time.
	 */
	static of(text: string): LineIndex {
		const starts = [0];

		for (let index = text.indexOf('\n'); index >= 0; index = text.indexOf('\n', index + 1)) {
			starts.push(index + 1);
		}

		return new LineIndex(starts);
	}

	/**
	 * Resolve the one-based line a source offset falls on.
	 *
	 * @param offset - The source offset.
	 * @returns The line number, clamped to the first line for a negative offset.
	 */
	lineAt(offset: number): number {
		let low = 0;
		let high = this.#starts.length - 1;

		while (low < high) {
			const middle = Math.ceil((low + high) / 2);

			if ((this.#starts[middle] ?? 0) <= offset) {
				low = middle;
			} else {
				high = middle - 1;
			}
		}

		return low + 1;
	}
}
