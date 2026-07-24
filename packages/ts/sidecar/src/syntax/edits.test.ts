import assert from 'node:assert/strict';
import { test } from 'node:test';
import { EditApplier } from '#sidecar/syntax/edits';

test('EditApplier.nonOverlapping drops overlapping edits and sorts by start', () => {
	const kept = new EditApplier().nonOverlapping([
		{ start: 10, end: 20, replacement: 'b' },
		{ start: 0, end: 5, replacement: 'a' },
		{ start: 15, end: 25, replacement: 'c' },
	]);

	assert.deepEqual(
		kept.map((edit) => {
			return edit.start;
		}),
		[0, 10],
	);
});
