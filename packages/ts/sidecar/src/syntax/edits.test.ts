import assert from 'node:assert/strict';
import { test } from 'node:test';
import { Edits } from '#sidecar/syntax/edits';

test('Edits.nonOverlapping drops overlapping edits and sorts by start', () => {
	const kept = Edits.nonOverlapping([
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
