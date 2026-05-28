import { parseSync } from 'oxc-parser';
import { getEnd, getStart, visit } from '#devx/ast';
import type { Edit, Node } from '#devx/types';

const STATEMENT_BODY_KEYS: Record<string, string[]> = {
	DoWhileStatement: ['body'],
	ForInStatement: ['body'],
	ForOfStatement: ['body'],
	ForStatement: ['body'],
	IfStatement: ['consequent', 'alternate'],
	WhileStatement: ['body'],
	WithStatement: ['body'],
};

function lineIndent(source: string, pos: number): string {
	const lineStart = source.lastIndexOf('\n', pos - 1) + 1;
	const match = source.slice(lineStart, pos).match(/^[ \t]*/);

	return match?.[0] ?? '';
}

function wrapStatementBody(source: string, owner: Node, body: Node): Edit | null {
	if (body.type === 'BlockStatement' || body.type === 'IfStatement') {
		return null;
	}

	const start = getStart(body);
	const end = getEnd(body);
	const ownerStart = getStart(owner);

	if (start < 0 || end < 0 || ownerStart < 0) {
		return null;
	}

	const indent = lineIndent(source, ownerStart);
	const bodySource = source.slice(start, end);

	return {
		start,
		end,
		replacement: `{\n${indent}\t${bodySource}\n${indent}}`,
	};
}

export function computeBodyWrapEdits(content: string, virtualName: string): Edit[] {
	const parsed = parseSync(virtualName, content) as unknown as { program: Node };
	const edits: Edit[] = [];

	visit(parsed.program, (node) => {
		const bodyKeys = STATEMENT_BODY_KEYS[node.type];

		if (!bodyKeys) {
			return;
		}

		for (const key of bodyKeys) {
			const body = node[key] as Node | undefined;

			if (!body) {
				continue;
			}

			const edit = wrapStatementBody(content, node, body);

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
