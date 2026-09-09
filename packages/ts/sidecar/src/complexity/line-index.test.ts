import assert from 'node:assert/strict';
import { test } from 'node:test';
import { LineIndex } from '#sidecar/complexity/line-index';

test('offsets resolve to the one-based line that contains them', () => {
	const index = LineIndex.of('one\ntwo\n\nfour');

	assert.equal(index.lineAt(0), 1);
	assert.equal(index.lineAt(3), 1);
	assert.equal(index.lineAt(4), 2);
	assert.equal(index.lineAt(8), 3);
	assert.equal(index.lineAt(9), 4);
	assert.equal(index.lineAt(12), 4);
});

test('an empty document is one line', () => {
	assert.equal(LineIndex.of('').lineAt(0), 1);
});

test('an offset before the start clamps to the first line', () => {
	assert.equal(LineIndex.of('one\ntwo').lineAt(-1), 1);
});
