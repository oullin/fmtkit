/**
 * An immutable source file: its virtual name paired with its text.
 *
 * The document answers the text-coordinate queries formatting passes need —
 * line starts, line indents, the inferred indent unit, and range slices —
 * without exposing a mutable buffer. It carries no caches: every query is
 * computed from the frozen text on demand.
 */
export class SourceDocument {
	/** The complete source text. */
	readonly text: string;

	/** The filename used to select parser syntax and label errors. */
	readonly virtualName: string;

	private constructor(virtualName: string, text: string) {
		this.virtualName = virtualName;
		this.text = text;

		Object.freeze(this);
	}

	/**
	 * Build a document from a virtual name and its source text.
	 *
	 * @param virtualName - The filename used to select parser syntax and label errors.
	 * @param text - The complete source text.
	 * @returns The immutable source document.
	 */
	static of(virtualName: string, text: string): SourceDocument {
		return new SourceDocument(virtualName, text);
	}

	/**
	 * Derive a document that keeps this name but carries new text.
	 *
	 * @param text - The replacement source text.
	 * @returns A new document over the same virtual name.
	 */
	withText(text: string): SourceDocument {
		return new SourceDocument(this.virtualName, text);
	}

	/**
	 * Find the start offset of the line containing a position.
	 *
	 * @param position - A source offset.
	 * @returns The offset immediately after the preceding newline, or zero.
	 */
	lineStart(position: number): number {
		return this.text.lastIndexOf('\n', position - 1) + 1;
	}

	/**
	 * Read the leading whitespace of the line containing a position.
	 *
	 * @param position - A source offset.
	 * @returns The line's leading spaces and tabs.
	 */
	lineIndent(position: number): string {
		const start = this.lineStart(position);
		const match = this.text.slice(start, position).match(/^[ \t]*/);

		return match?.[0] ?? '';
	}

	/**
	 * Infer the file's per-level indentation unit from its content.
	 *
	 * The unit is read relative to the content's baseline (minimum) indentation
	 * so that embedded blocks whose whole body sits below column zero — an HTML
	 * `<script>` body or a list-nested Markdown fence — report one nesting level
	 * rather than their absolute baseline. The baseline is the smallest leading
	 * whitespace across non-blank, non-comment-continuation lines; the unit is
	 * the first deeper line's indentation with that baseline prefix removed. This
	 * leaves column-zero source (baseline of none) behaving as a plain first-
	 * indent read. Content with no line below its baseline falls back to a tab.
	 *
	 * @returns The detected indent unit, or a tab when none can be inferred.
	 */
	indentUnit(): string {
		const indents: string[] = [];

		for (const line of this.text.split('\n')) {
			const match = line.match(/^([ \t]*)(\S)/);

			if (!match || match[2] === '*') {
				continue;
			}

			indents.push(match[1] ?? '');
		}

		if (indents.length === 0) {
			return '\t';
		}

		const baseline = indents.reduce((shortest, indent) => {
			return indent.length < shortest.length ? indent : shortest;
		});

		for (const indent of indents) {
			if (indent.length > baseline.length) {
				return indent.slice(baseline.length);
			}
		}

		return '\t';
	}

	/**
	 * Slice a range out of the source text.
	 *
	 * @param start - The inclusive start offset.
	 * @param end - The exclusive end offset.
	 * @returns The selected source text.
	 */
	slice(start: number, end: number): string {
		return this.text.slice(start, end);
	}
}
