import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isDeclarationFile, isTargetFile } from '#sidecar/file-targets';

test('isTargetFile accepts ts and vue but not declarations', () => {
	assert.equal(isTargetFile('app.ts'), true);

	assert.equal(isTargetFile('widget.vue'), true);

	assert.equal(isTargetFile('types.d.ts'), false);

	assert.equal(isTargetFile('notes.md'), false);
});

test('isDeclarationFile only matches .d.ts', () => {
	assert.equal(isDeclarationFile('types.d.ts'), true);

	assert.equal(isDeclarationFile('app.ts'), false);
});
