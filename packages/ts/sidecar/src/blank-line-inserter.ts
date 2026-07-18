import { Ast } from '#sidecar/ast';
import { isErr } from '#sidecar/result';
import { Rules } from '#sidecar/rules';
import { Sources } from '#sidecar/sources';

/** Computes and applies the blank lines required by formatter rules. */
export class BlankLines {
	static #countNewlines(source: string, from: number, to: number): number {
		let count = 0;

		for (let i = from; i < to; i++) {
			if (source.charCodeAt(i) === 10) {
				count++;
			}
		}

		return count;
	}

	/**
	 * Compute positions where a blank line must be inserted.
	 *
	 * @param content - The source text to inspect.
	 * @param virtualName - The filename used to parse the source.
	 * @returns Source offsets where one newline should be inserted.
	 */
	static computeInsertPositions(content: string, virtualName: string): number[] {
		const parsed = Sources.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const lists = Ast.collectStatementLists(parsed.value.program);
		const positions: number[] = [];

		for (const list of lists) {
			for (let i = 1; i < list.length; i++) {
				const prev = list[i - 1];
				const next = list[i];

				if (!prev || !next) {
					continue;
				}

				if (!Rules.needsBlankLine(prev, next)) {
					continue;
				}

				const prevEnd = Ast.getEnd(prev);
				const nextStart = Ast.getStart(next);

				if (prevEnd < 0 || nextStart < 0 || nextStart <= prevEnd) {
					continue;
				}

				if (BlankLines.#countNewlines(content, prevEnd, nextStart) >= 2) {
					continue;
				}

				const lineStart = content.lastIndexOf('\n', nextStart - 1);

				if (lineStart < 0) {
					continue;
				}

				positions.push(lineStart + 1);
			}
		}

		return positions;
	}

	/**
	 * Insert blank lines at precomputed source offsets.
	 *
	 * @param content - The source text to update.
	 * @param positions - The source offsets where one newline should be inserted.
	 * @returns The source with the requested blank lines inserted.
	 */
	static insert(content: string, positions: number[]): string {
		const sorted = [...new Set(positions)].sort((a, b) => {
			return b - a;
		});

		let out = content;

		for (const pos of sorted) {
			out = out.slice(0, pos) + '\n' + out.slice(pos);
		}

		return out;
	}
}
