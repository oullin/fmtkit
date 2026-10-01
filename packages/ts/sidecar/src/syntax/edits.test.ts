import assert from 'node:assert/strict';
import { test } from 'node:test';
import { EditApplier } from '#sidecar/syntax/edits';

test('EditApplier.nonOverlapping drops overlapping edits and sorts by start', () => {
	const kept = new EditApplier().nonOverlapping([
		{ end: 20, replacement: 'b', start: 10 },
		{ end: 5, replacement: 'a', start: 0 },
		{ end: 25, replacement: 'c', start: 15 },
	]);

	assert.deepEqual(
		kept.map((edit) => {
			return edit.start;
		}),
		[0, 10],
	);
});
