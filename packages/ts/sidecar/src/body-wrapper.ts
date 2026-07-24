import { AstReader } from '#sidecar/syntax/ast-reader';
import { isErr } from '#sidecar/kernel/result';
import { SourceDocument } from '#sidecar/syntax/source-document';
import { SourceParser } from '#sidecar/syntax/source-parser';
import type { Edit } from '#sidecar/syntax/edits';
import type { Node } from '#sidecar/syntax/node-schema';

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
export class BodyWrapper {
	static readonly #ast = new AstReader();

	static readonly #parser = new SourceParser();

	static #wrapStatementBody(document: SourceDocument, owner: Node, body: Node, indentUnit: string): Edit | null {
		if (body.type === 'BlockStatement') {
			return null;
		}

		if (body.type === 'IfStatement' && owner.type === 'IfStatement' && owner.alternate === body) {
			return null;
		}

		const start = BodyWrapper.#ast.getStart(body);
		const end = BodyWrapper.#ast.getEnd(body);
		const ownerStart = BodyWrapper.#ast.getStart(owner);

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

	/**
	 * Compute edits that wrap unbraced statement bodies.
	 *
	 * @param content - The source text to inspect.
	 * @param virtualName - The filename used to parse the source.
	 * @returns Non-overlapping body-wrap edits, or none for invalid source.
	 */
	static computeEdits(content: string, virtualName: string): Edit[] {
		const parsed = BodyWrapper.#parser.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const document = SourceDocument.of(virtualName, content);
		const edits: Edit[] = [];
		const indentUnit = document.indentUnit();

		BodyWrapper.#ast.visit(parsed.value.program, (node) => {
			const bodyKeys = STATEMENT_BODY_KEYS[node.type];

			if (!bodyKeys) {
				return;
			}

			for (const key of bodyKeys) {
				const body = BodyWrapper.#ast.childNode(node, key);

				if (!body) {
					continue;
				}

				const edit = BodyWrapper.#wrapStatementBody(document, node, body, indentUnit);

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
}
