import type { AstReader } from '#sidecar/syntax/ast-reader';
import { isErr } from '#sidecar/kernel/result';
import type { SourceParser } from '#sidecar/syntax/source-parser';
import type { StatementSpacingPolicy } from '#sidecar/passes/policies/statement-spacing-policy';
import type { Edit } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { SourceDocument } from '#sidecar/syntax/source-document';

/** Inserts the blank lines the formatter's statement-spacing rules require. */
export class BlankLinePass implements FormattingPass {
	/** The pass identity used for reporting. */
	readonly name = 'blank-lines';

	readonly #parser: SourceParser;
	readonly #ast: AstReader;
	readonly #spacing: StatementSpacingPolicy;

	/**
	 * @param dependencies - The syntax services and policy consumed by the pass.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 * @param dependencies.spacing - Decides which statement pairs need a blank line.
	 */
	constructor(dependencies: { parser: SourceParser; ast: AstReader; spacing: StatementSpacingPolicy }) {
		this.#parser = dependencies.parser;
		this.#ast = dependencies.ast;
		this.#spacing = dependencies.spacing;
	}

	/**
	 * Compute the zero-width newline inserts required by statement spacing.
	 *
	 * Each required blank line is a zero-width insert of a single newline at the
	 * start of the following statement's line. Positions are deduplicated so two
	 * statements that share a physical line contribute one newline, matching the
	 * former position-set insertion exactly.
	 *
	 * @param document - The document to inspect.
	 * @returns Zero-width newline inserts, or none for invalid source.
	 */
	computeEdits(document: SourceDocument): Edit[] {
		const parsed = this.#parser.parse(document.virtualName, document.text);

		if (isErr(parsed)) {
			return [];
		}

		const content = document.text;
		const lists = this.#ast.collectStatementLists(parsed.value.program);
		const positions = new Set<number>();

		for (const list of lists) {
			for (let i = 1; i < list.length; i++) {
				const prev = list[i - 1];
				const next = list[i];

				if (!prev || !next) {
					continue;
				}

				if (!this.#spacing.needsBlankLine(prev, next)) {
					continue;
				}

				const prevEnd = this.#ast.getEnd(prev);
				const nextStart = this.#ast.getStart(next);

				if (prevEnd < 0 || nextStart < 0 || nextStart <= prevEnd) {
					continue;
				}

				if (this.#countNewlines(content, prevEnd, nextStart) >= 2) {
					continue;
				}

				const lineStart = content.lastIndexOf('\n', nextStart - 1);

				if (lineStart < 0) {
					continue;
				}

				positions.add(lineStart + 1);
			}
		}

		return [...positions].map((position) => {
			return { start: position, end: position, replacement: '\n' };
		});
	}

	#countNewlines(source: string, from: number, to: number): number {
		let count = 0;

		for (let i = from; i < to; i++) {
			if (source.charCodeAt(i) === 10) {
				count++;
			}
		}

		return count;
	}
}
