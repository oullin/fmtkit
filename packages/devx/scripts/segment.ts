import { computeInsertPositions, insertBlankLines } from './blank-line-inserter';
import { computeReorderEdits } from './class-reorder';
import { applyEdits } from './edits';

export function processSegment(content: string, virtualName: string): string {
	const reorderEdits = computeReorderEdits(content, virtualName);
	const reordered = reorderEdits.length > 0 ? applyEdits(content, reorderEdits) : content;
	const positions = computeInsertPositions(reordered, virtualName, 0);

	return insertBlankLines(reordered, positions);
}
