import type { Edit } from '#sidecar/types';

/** Applies source edits in offset-safe order. */
export class Edits {
	/**
	 * Apply edits to source text from the highest offset to the lowest.
	 *
	 * @param source - The original source text.
	 * @param edits - The edits expressed against the original offsets.
	 * @returns The edited source text.
	 */
	static apply(source: string, edits: Edit[]): string {
		const sorted = [...edits].sort((a, b) => {
			return b.start - a.start;
		});

		let out = source;

		for (const edit of sorted) {
			out = out.slice(0, edit.start) + edit.replacement + out.slice(edit.end);
		}

		return out;
	}
}
