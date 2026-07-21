import { Ast } from '#sidecar/ast';
import { isErr } from '#sidecar/result';
import { SourceText } from '#sidecar/source-text';
import { Sources } from '#sidecar/sources';
import type { Edit, Node } from '#sidecar/types';

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
	static #wrapStatementBody(source: string, owner: Node, body: Node, indentUnit: string): Edit | null {
		if (body.type === 'BlockStatement') {
			return null;
		}

		if (body.type === 'IfStatement' && owner.type === 'IfStatement' && owner.alternate === body) {
			return null;
		}

		const start = Ast.getStart(body);
		const end = Ast.getEnd(body);
		const ownerStart = Ast.getStart(owner);

		if (start < 0 || end < 0 || ownerStart < 0) {
			return null;
		}

		const indent = SourceText.lineIndent(source, ownerStart);
		const bodySource = source.slice(start, end);

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
		const parsed = Sources.parse(virtualName, content);

		if (isErr(parsed)) {
			return [];
		}

		const edits: Edit[] = [];
		const indentUnit = SourceText.detectIndentUnit(content);

		Ast.visit(parsed.value.program, (node) => {
			const bodyKeys = STATEMENT_BODY_KEYS[node.type];

			if (!bodyKeys) {
				return;
			}

			for (const key of bodyKeys) {
				const body = Ast.childNode(node, key);

				if (!body) {
					continue;
				}

				const edit = BodyWrapper.#wrapStatementBody(content, node, body, indentUnit);

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
