import type { Edit } from '#sidecar/types';

export function applyEdits(source: string, edits: Edit[]): string {
	const sorted = [...edits].sort((a, b) => {
		return b.start - a.start;
	});

	let out = source;

	for (const edit of sorted) {
		out = out.slice(0, edit.start) + edit.replacement + out.slice(edit.end);
	}

	return out;
}
