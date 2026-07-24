/** A source replacement expressed against the original text offsets. */
export type Edit = {
	/** The inclusive replacement start. */
	start: number;

	/** The exclusive replacement end. */
	end: number;

	/** The text inserted in place of the selected range. */
	replacement: string;
};

/** Applies source edits in offset-safe order. */
export class Edits {
	/**
	 * Report whether two edit ranges overlap.
	 *
	 * @param a - The first edit range.
	 * @param b - The second edit range.
	 * @returns `true` when the edit ranges intersect.
	 */
	static rangesOverlap(a: Edit, b: Edit): boolean {
		return a.start < b.end && b.start < a.end;
	}

	/**
	 * Keep a deterministic set of non-overlapping edits.
	 *
	 * @param edits - Candidate edits expressed against the same source text.
	 * @returns Accepted edits sorted from the lowest offset to the highest.
	 */
	static nonOverlapping(edits: Edit[]): Edit[] {
		const accepted: Edit[] = [];

		const sorted = [...edits].sort((a, b) => {
			return a.start - b.start || b.end - b.start - (a.end - a.start);
		});

		for (const edit of sorted) {
			if (accepted.some((existing) => Edits.rangesOverlap(existing, edit))) {
				continue;
			}

			accepted.push(edit);
		}

		return accepted.sort((a, b) => {
			return a.start - b.start;
		});
	}

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
