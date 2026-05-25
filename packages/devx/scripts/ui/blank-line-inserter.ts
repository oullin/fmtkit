import { parseSync } from 'oxc-parser';
import { collectStatementLists, getEnd, getStart } from '@ui/ast';
import { needsBlankLine } from '@ui/rules';
import type { Node } from '@ui/types';

function countNewlines(source: string, from: number, to: number): number {
	let count = 0;

	for (let i = from; i < to; i++) {
		if (source.charCodeAt(i) === 10) {
			count++;
		}
	}

	return count;
}

export function computeInsertPositions(content: string, virtualName: string, baseOffset: number): number[] {
	const parsed = parseSync(virtualName, content) as unknown as { program: Node };
	const lists = collectStatementLists(parsed.program);
	const positions: number[] = [];

	for (const list of lists) {
		for (let i = 1; i < list.length; i++) {
			const prev = list[i - 1];
			const next = list[i];

			if (!needsBlankLine(prev, next)) {
				continue;
			}

			const prevEnd = getEnd(prev);
			const nextStart = getStart(next);

			if (prevEnd < 0 || nextStart < 0 || nextStart <= prevEnd) {
				continue;
			}

			if (countNewlines(content, prevEnd, nextStart) >= 2) {
				continue;
			}

			const lineStart = content.lastIndexOf('\n', nextStart - 1);

			if (lineStart < 0) {
				continue;
			}

			positions.push(lineStart + 1 + baseOffset);
		}
	}

	return positions;
}

export function insertBlankLines(content: string, positions: number[]): string {
	const sorted = [...new Set(positions)].sort((a, b) => b - a);
	let out = content;

	for (const pos of sorted) {
		out = out.slice(0, pos) + '\n' + out.slice(pos);
	}

	return out;
}
