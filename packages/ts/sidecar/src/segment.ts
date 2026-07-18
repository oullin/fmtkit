import { BlankLines } from '#sidecar/blank-line-inserter';
import { BodyWrapper } from '#sidecar/body-wrapper';
import { ClassReorder } from '#sidecar/class-reorder';
import { DeclarationReorder } from '#sidecar/declaration-reorder';
import { Edits } from '#sidecar/edits';

function applyBodyWraps(content: string, virtualName: string): string {
	let current = content;

	for (let i = 0; i < 5; i++) {
		const edits = BodyWrapper.computeEdits(current, virtualName);

		if (edits.length === 0) {
			return current;
		}

		current = Edits.apply(current, edits);
	}

	return current;
}

export function processSegment(content: string, virtualName: string): string {
	const bodyWrapped = applyBodyWraps(content, virtualName);
	const classReorderEdits = ClassReorder.computeEdits(bodyWrapped, virtualName);
	const classReordered = classReorderEdits.length > 0 ? Edits.apply(bodyWrapped, classReorderEdits) : bodyWrapped;
	const declarationReorderEdits = DeclarationReorder.computeEdits(classReordered, virtualName);
	const reordered = declarationReorderEdits.length > 0 ? Edits.apply(classReordered, declarationReorderEdits) : classReordered;
	const positions = BlankLines.computeInsertPositions(reordered, virtualName);

	return BlankLines.insert(reordered, positions);
}
