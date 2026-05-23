import { computeInsertPositions, insertBlankLines } from '@ui/blank-line-inserter';
import { computeReorderEdits } from '@ui/class-reorder';
import { applyEdits } from '@ui/edits';

export function processSegment(content: string, virtualName: string): string {
	const reorderEdits = computeReorderEdits(content, virtualName);
	const reordered = reorderEdits.length > 0 ? applyEdits(content, reorderEdits) : content;
	const positions = computeInsertPositions(reordered, virtualName, 0);

	return insertBlankLines(reordered, positions);
}
