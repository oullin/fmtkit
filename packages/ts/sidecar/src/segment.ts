import { computeInsertPositions, insertBlankLines } from '#devx/blank-line-inserter';
import { computeBodyWrapEdits } from '#devx/body-wrapper';
import { computeReorderEdits } from '#devx/class-reorder';
import { computeDeclarationReorderEdits } from '#devx/declaration-reorder';
import { applyEdits } from '#devx/edits';

function applyBodyWraps(content: string, virtualName: string): string {
	let current = content;

	for (let i = 0; i < 5; i++) {
		const edits = computeBodyWrapEdits(current, virtualName);

		if (edits.length === 0) {
			return current;
		}

		current = applyEdits(current, edits);
	}

	return current;
}

export function processSegment(content: string, virtualName: string): string {
	const bodyWrapped = applyBodyWraps(content, virtualName);
	const classReorderEdits = computeReorderEdits(bodyWrapped, virtualName);
	const classReordered = classReorderEdits.length > 0 ? applyEdits(bodyWrapped, classReorderEdits) : bodyWrapped;
	const declarationReorderEdits = computeDeclarationReorderEdits(classReordered, virtualName);
	const reordered = declarationReorderEdits.length > 0 ? applyEdits(classReordered, declarationReorderEdits) : classReordered;
	const positions = computeInsertPositions(reordered, virtualName);

	return insertBlankLines(reordered, positions);
}
