import { childNode, getEnd, getStart, visit } from '#sidecar/ast';
import { lineIndent, parseCleanly } from '#sidecar/pass-utils';
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

function wrapStatementBody(source: string, owner: Node, body: Node): Edit | null {
	if (body.type === 'BlockStatement') {
		return null;
	}

	if (body.type === 'IfStatement' && owner.type === 'IfStatement' && owner.alternate === body) {
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
	const parsed = parseCleanly(virtualName, content);

	if (!parsed) {
		return [];
	}

	const edits: Edit[] = [];

	visit(parsed.program, (node) => {
		const bodyKeys = STATEMENT_BODY_KEYS[node.type];

		if (!bodyKeys) {
			return;
		}

		for (const key of bodyKeys) {
			const body = childNode(node, key);

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
