import type { AstReader } from '#sidecar/syntax/ast-reader';
import { isErr } from '#sidecar/kernel/result';
import type { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';
import type { FormattingPass } from '#sidecar/passes/pass';
import type { Node } from '#sidecar/syntax/node-schema';
import type { SourceDocument } from '#sidecar/syntax/source-document';

const STATEMENT_BODY_KEYS: Record<string, string[]> = {
	DoWhileStatement: ['body'],
	ForInStatement: ['body'],
	ForOfStatement: ['body'],
	ForStatement: ['body'],
	IfStatement: ['consequent', 'alternate'],
	WhileStatement: ['body'],
	WithStatement: ['body'],
};

/** Wraps unbraced statement bodies without changing unparsable source. */
export class BodyWrapPass implements FormattingPass {
	/** The pass identity used for reporting. */
	readonly name = 'body-wrap';

	readonly #parser: SourceParser;
	readonly #ast: AstReader;

	/**
	 * @param dependencies - The syntax services consumed by the pass.
	 * @param dependencies.parser - Parses source into a trustworthy tree.
	 * @param dependencies.ast - Traverses and reads validated node fields.
	 */
	constructor(dependencies: { parser: SourceParser; ast: AstReader }) {
		this.#parser = dependencies.parser;
		this.#ast = dependencies.ast;
	}

	/**
	 * Compute edits that wrap unbraced statement bodies.
	 *
	 * @param document - The document to inspect.
	 * @returns Non-overlapping body-wrap edits, or none for invalid source.
	 */
	computeEdits(document: SourceDocument): Edit[] {
		const parsed = this.#parser.parse(document.virtualName, document.text);

		if (isErr(parsed)) {
			return [];
		}

		const edits: Edit[] = [];
		const indentUnit = document.indentUnit();

		this.#ast.visit(parsed.value.program, (node) => {
			const bodyKeys = STATEMENT_BODY_KEYS[node.type];

			if (!bodyKeys) {
				return;
			}

			for (const key of bodyKeys) {
				const body = this.#ast.childNode(node, key);

				if (!body) {
					continue;
				}

				const edit = this.#wrapStatementBody(document, node, body, indentUnit);

				if (edit) {
					edits.push(edit);
				}
			}
		});

		return edits
			.sort((a, b) => {
				return a.start - b.start || b.end - b.start - (a.end - a.start);
			})
			.filter((edit, index, sorted) => {
				return !sorted.some((other, otherIndex) => {
					return otherIndex < index && edit.start < other.end;
				});
			});
	}

	#wrapStatementBody(document: SourceDocument, owner: Node, body: Node, indentUnit: string): Edit | null {
		if (body.type === 'BlockStatement') {
			return null;
		}

		if (body.type === 'IfStatement' && owner.type === 'IfStatement' && owner.alternate === body) {
			return null;
		}

		const start = this.#ast.getStart(body);
		const end = this.#ast.getEnd(body);
		const ownerStart = this.#ast.getStart(owner);

		if (start < 0 || end < 0 || ownerStart < 0) {
			return null;
		}

		const indent = document.lineIndent(ownerStart);
		const bodySource = document.slice(start, end);

		return {
			start,
			end,
			replacement: `{\n${indent}${indentUnit}${bodySource}\n${indent}}`,
		};
	}
}
